//! Project-wide symbol table (US-203 second half).
//!
//! `validate_native` is called once per file, but a `package` declared in
//! `engine.sysml` should be visible from `vehicle.sysml` when the latter
//! imports it. `ProjectIndex` is built in a pre-pass over every file the
//! user gave on the CLI, then handed to each per-file validation.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;
#[cfg(test)]
use std::path::Path;

use crate::ast;
use crate::lex::{Scanner, Token};
use crate::library::extract_declarations_for_project_index;

#[derive(Clone, Debug, Default)]
pub struct ProjectIndex {
    /// Every fully-qualified declared name found in any user file —
    /// `MyPackage::Engine`, `MyPackage::Sub::Foo`, etc.
    qualified_names: HashSet<String>,
    /// Every unqualified declared name — `Engine`, `Foo`, etc.
    unqualified_names: HashSet<String>,
    /// Map from package name to the set of unqualified members declared
    /// inside it. Used to resolve `import MyPackage::*;` wildcards.
    package_members: HashMap<String, HashSet<String>>,
    /// Specialization edges: `child -> [parent, parent, ...]` keyed on
    /// unqualified declared name. Populated from `:>` and `specializes`
    /// declarations across every project file. Used by SYSML220 to
    /// detect cycles project-wide.
    specialization_edges: HashMap<String, Vec<String>>,
    /// File paths whose tokens contributed to the index (for debugging
    /// and future `--show-index` output).
    files: Vec<PathBuf>,
}

impl ProjectIndex {
    #[allow(dead_code)] // Used in tests and reserved for project-manifest support (US-204).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Tokenize every file and merge its declarations into the index. A
    /// file that fails to tokenize is skipped silently; the per-file
    /// validation pass will surface its diagnostics.
    pub fn build(files: &[PathBuf]) -> Self {
        let mut index = Self::default();
        for path in files {
            let Ok(text) = fs::read_to_string(path) else {
                continue;
            };
            let scan = Scanner::new(path, &text).scan();
            let (package, declarations) =
                extract_declarations_for_project_index(&scan.tokens);
            index.files.push(path.clone());
            for declaration in &declarations {
                index.unqualified_names.insert(declaration.clone());
                if let Some(package) = &package {
                    index
                        .qualified_names
                        .insert(format!("{package}::{declaration}"));
                    index
                        .package_members
                        .entry(package.clone())
                        .or_default()
                        .insert(declaration.clone());
                }
            }
            // Batch K: also collect declarations via tree-sitter so the
            // project-wide index sees metadata-tag declarations and any
            // other shapes the token recognizer misses. AST-collected
            // names go into the unqualified set and (when a package was
            // detected) the qualified set + package members.
            if let Some(ast_parse) = ast::parse(&text) {
                let ast_names = ast::collect_declared_names(&ast_parse);
                for name in ast_names {
                    index.unqualified_names.insert(name.clone());
                    if let Some(package) = &package {
                        index
                            .qualified_names
                            .insert(format!("{package}::{name}"));
                        index
                            .package_members
                            .entry(package.clone())
                            .or_default()
                            .insert(name);
                    }
                }
            }
            // Walk the token stream a second time to harvest
            // specialization edges. This is cheap: same tokens, no I/O.
            harvest_specialization_edges(&scan.tokens, &mut index.specialization_edges);
        }
        index
    }

    pub fn contains_qualified(&self, name: &str) -> bool {
        self.qualified_names.contains(name)
    }

    pub fn contains_unqualified(&self, name: &str) -> bool {
        self.unqualified_names.contains(name)
    }

    /// Resolve `<namespace>::<leaf>` against the project's package map.
    /// Returns true if the project declares a package named `namespace`
    /// containing a direct member `leaf`.
    pub fn namespace_contains(&self, namespace: &str, leaf: &str) -> bool {
        self.package_members
            .get(namespace)
            .is_some_and(|members| members.contains(leaf))
    }

    #[allow(dead_code)] // Phase 2+ LSP/project-info consumer.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn specialization_parents(&self, child: &str) -> &[String] {
        self.specialization_edges
            .get(child)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Walk every declaration in the index and look for a cycle through
    /// the specialization graph. Returns the first cycle found as the
    /// list of declaration names traversed (in order), or `None` if the
    /// graph is acyclic.
    ///
    /// Uses iterative DFS with a visited-stack so deeply nested graphs
    /// don't blow the call stack. Project-wide scope: cycles across
    /// file boundaries are detected.
    pub fn find_specialization_cycle(&self) -> Option<Vec<String>> {
        let mut globally_visited: HashSet<String> = HashSet::new();
        for start in self.specialization_edges.keys() {
            if globally_visited.contains(start) {
                continue;
            }
            if let Some(cycle) = self.find_cycle_from(start, &mut globally_visited) {
                return Some(cycle);
            }
        }
        None
    }

    fn find_cycle_from(
        &self,
        start: &str,
        globally_visited: &mut HashSet<String>,
    ) -> Option<Vec<String>> {
        // DFS with explicit stack. Track the current DFS path so we can
        // reconstruct the cycle when we revisit a node.
        let mut stack: VecDeque<(String, usize)> = VecDeque::new();
        let mut path: Vec<String> = Vec::new();
        let mut on_path: HashSet<String> = HashSet::new();

        stack.push_back((start.to_string(), 0));
        path.push(start.to_string());
        on_path.insert(start.to_string());

        while let Some((node, edge_index)) = stack.back().cloned() {
            let parents = self.specialization_parents(&node);
            if edge_index >= parents.len() {
                // Done with this node.
                stack.pop_back();
                if let Some(name) = path.pop() {
                    on_path.remove(&name);
                    globally_visited.insert(name);
                }
                continue;
            }
            // Advance edge_index for the current frame.
            if let Some(frame) = stack.back_mut() {
                frame.1 = edge_index + 1;
            }
            let next = parents[edge_index].clone();
            if on_path.contains(&next) {
                // Cycle: extract path from `next` to end + closing edge.
                let cycle_start = path.iter().position(|name| name == &next)?;
                let mut cycle = path[cycle_start..].to_vec();
                cycle.push(next);
                return Some(cycle);
            }
            if globally_visited.contains(&next) {
                continue;
            }
            stack.push_back((next.clone(), 0));
            path.push(next.clone());
            on_path.insert(next);
        }
        None
    }
}

/// Walk tokens and record every `:>` / `specializes` edge encountered.
/// Edges are keyed on the most recently seen declared name (the child)
/// and accumulate all its specialization parents.
fn harvest_specialization_edges(
    tokens: &[Token],
    edges: &mut HashMap<String, Vec<String>>,
) {
    use crate::lex::TokenKind;

    // Track the most recently declared name so we can attach edges to it.
    // A declaration is recognized by the pattern produced by
    // `extract_declarations_for_project_index`; for the cycle-detection
    // use case, "name immediately followed by `:>`" is the right signal.
    let mut last_decl: Option<String> = None;
    let mut cursor = 0;
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        match token.kind {
            TokenKind::Identifier => {
                // Speculative: this might be a declared name. Confirmed by
                // a subsequent `:>` or `specializes`.
                last_decl = Some(token.value.clone());
                cursor += 1;
            }
            _ => {
                let is_spec_marker = token.value == ":>" || token.value == "specializes";
                if is_spec_marker {
                    if let (Some(child), Some(parent_token)) =
                        (last_decl.as_ref(), tokens.get(cursor + 1))
                    {
                        if parent_token.kind == TokenKind::Identifier {
                            // Read the qualified-name tail (`A::B::C`) and
                            // store just the leaf — the project's symbol
                            // index keys on unqualified names. This is a
                            // simplification; full qualified-edge tracking
                            // is a Phase 2.x follow-up.
                            let leaf = read_qualified_leaf(tokens, cursor + 1);
                            edges
                                .entry(child.clone())
                                .or_default()
                                .push(leaf);
                        }
                    }
                }
                cursor += 1;
            }
        }
    }
}

fn read_qualified_leaf(tokens: &[Token], start: usize) -> String {
    use crate::lex::TokenKind;
    let mut leaf = String::new();
    let mut cursor = start;
    while let Some(token) = tokens.get(cursor) {
        if token.kind == TokenKind::Identifier {
            leaf = token.value.clone();
            cursor += 1;
            if tokens.get(cursor).map(|t| t.value.as_str()) == Some("::") {
                cursor += 1;
                continue;
            }
        }
        break;
    }
    leaf
}

/// Convenience: build a one-file index. Used by tests that need a
/// project context without actually walking the filesystem.
#[cfg(test)]
impl ProjectIndex {
    pub fn from_tokens(
        package: Option<String>,
        declarations: Vec<String>,
        file: &Path,
    ) -> Self {
        let mut index = Self::default();
        index.files.push(file.to_path_buf());
        for declaration in &declarations {
            index.unqualified_names.insert(declaration.clone());
            if let Some(package) = &package {
                index
                    .qualified_names
                    .insert(format!("{package}::{declaration}"));
                index
                    .package_members
                    .entry(package.clone())
                    .or_default()
                    .insert(declaration.clone());
            }
        }
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(contents: &str, suffix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = env::temp_dir().join(format!("sysml_validate_project_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(format!("model{suffix}"));
        fs::write(&path, contents).expect("write");
        path
    }

    #[test]
    fn indexes_declarations_across_files() {
        let a = temp_file("package A { part def Engine; }", ".sysml");
        let b = temp_file("package B { part def Wheel; }", ".sysml");
        let index = ProjectIndex::build(&[a.clone(), b.clone()]);
        assert!(index.contains_unqualified("Engine"));
        assert!(index.contains_unqualified("Wheel"));
        assert!(index.contains_qualified("A::Engine"));
        assert!(index.contains_qualified("B::Wheel"));
        assert!(!index.contains_qualified("A::Wheel"));
        let _ = fs::remove_file(&a);
        let _ = fs::remove_file(&b);
    }

    #[test]
    fn namespace_membership_lookup() {
        let a = temp_file("package Vehicles { part def Engine; part def Wheel; }", ".sysml");
        let index = ProjectIndex::build(&[a.clone()]);
        assert!(index.namespace_contains("Vehicles", "Engine"));
        assert!(index.namespace_contains("Vehicles", "Wheel"));
        assert!(!index.namespace_contains("Vehicles", "Other"));
        assert!(!index.namespace_contains("OtherPackage", "Engine"));
        let _ = fs::remove_file(&a);
    }

    #[test]
    fn empty_project_is_empty() {
        let index = ProjectIndex::empty();
        assert!(!index.contains_qualified("X"));
        assert!(!index.contains_unqualified("X"));
        assert!(!index.namespace_contains("X", "Y"));
    }

    // ---- Batch H: cycle detection (SYSML220) ----

    #[test]
    fn detects_two_node_specialization_cycle() {
        // A :> B; B :> A — classic cycle, intentionally split across two
        // files to exercise cross-file detection.
        let a = temp_file("package P { part def A :> B; }", ".sysml");
        let b = temp_file("package P { part def B :> A; }", ".sysml");
        let index = ProjectIndex::build(&[a.clone(), b.clone()]);
        let cycle = index
            .find_specialization_cycle()
            .expect("expected a cycle");
        assert!(cycle.contains(&"A".to_string()));
        assert!(cycle.contains(&"B".to_string()));
        let _ = fs::remove_file(&a);
        let _ = fs::remove_file(&b);
    }

    #[test]
    fn acyclic_specialization_chain_is_clean() {
        let a = temp_file(
            "package P { part def Animal; part def Mammal :> Animal; part def Dog :> Mammal; }",
            ".sysml",
        );
        let index = ProjectIndex::build(&[a.clone()]);
        assert!(index.find_specialization_cycle().is_none());
        let _ = fs::remove_file(&a);
    }

    #[test]
    fn detects_three_node_cycle() {
        let a = temp_file(
            "package P { part def A :> C; part def B :> A; part def C :> B; }",
            ".sysml",
        );
        let index = ProjectIndex::build(&[a.clone()]);
        assert!(index.find_specialization_cycle().is_some());
        let _ = fs::remove_file(&a);
    }
}
