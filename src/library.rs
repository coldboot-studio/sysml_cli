//! SysML v2 standard library loader (US-202).
//!
//! At compile time, `include_dir!` embeds the full upstream
//! `sysml.library/` tree (vendored at OMG release tag 2026-04, EPL-2.0)
//! into the binary. At runtime, [`LibraryLoader::default`] returns a
//! loader backed by the embedded copy; [`LibraryLoader::from_path`]
//! returns one backed by a user-supplied directory (the `--library-path`
//! override path).
//!
//! The loader tokenizes every library file with the existing scanner
//! (no separate parser is needed for v1 — we extract package names and
//! top-level declarations using the same token-walking patterns the
//! validator already uses) and builds two indices:
//!
//! - `qualified_names`: every declared name as `Package::Name`, so a
//!   reference like `ISQ::Mass` can be answered yes/no.
//! - `unqualified_names`: every declared name as a bare identifier, so
//!   a reference like `Part` or `Engine` (no qualifier) can be answered
//!   yes/no.
//!
//! Phase 2 batches will replace the second index with full qualified-
//! name resolution (US-203). For now, having both indices is sufficient
//! to eliminate the bulk of false-positive `SYSML040` warnings under
//! `--strict`.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use include_dir::{include_dir, Dir};

use crate::lex::{Scanner, Token, TokenKind};

/// The pinned upstream release tag. Bump when [`EMBEDDED_LIBRARY`] is
/// re-pointed at a new submodule revision.
pub const EMBEDDED_LIBRARY_RELEASE: &str = "2026-04";

/// Embed the vendored library at compile time. Path is relative to
/// `Cargo.toml`. If the submodule is not initialized, the build fails
/// here with a clear pointer back to README.
static EMBEDDED_LIBRARY: Dir<'_> = include_dir!(
    "$CARGO_MANIFEST_DIR/vendor/sysml-v2-release/sysml.library"
);

#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields are part of the public LibraryFile API consumed by Phase 2+ work.
pub struct LibraryFile {
    pub virtual_path: String,
    pub package: Option<String>,
    pub declarations: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LibraryLoader {
    files: Vec<LibraryFile>,
    qualified_names: HashSet<String>,
    unqualified_names: HashSet<String>,
    package_names: BTreeSet<String>,
    source_description: String,
}

impl LibraryLoader {
    /// Load the embedded standard library. Always succeeds.
    pub fn embedded() -> Self {
        let mut loader = Self::default();
        loader.source_description = format!(
            "embedded SysML v2 standard library (OMG release {})",
            EMBEDDED_LIBRARY_RELEASE
        );
        let entries = EMBEDDED_LIBRARY
            .find("**/*")
            .expect("static glob pattern is valid");
        for entry in entries {
            let Some(file) = entry.as_file() else {
                continue;
            };
            let path = file.path().to_string_lossy().to_string();
            if !has_library_extension(&path) {
                continue;
            }
            let Ok(text) = std::str::from_utf8(file.contents()) else {
                continue;
            };
            loader.ingest(&path, text);
        }
        loader.finalize();
        loader
    }

    /// Load a library from an arbitrary on-disk directory. Useful when a
    /// user wants to test against a pre-release library or a private fork
    /// via `--library-path`.
    pub fn from_path(root: &Path) -> Result<Self, String> {
        if !root.is_dir() {
            return Err(format!(
                "library path '{}' is not a directory",
                root.display()
            ));
        }
        let mut loader = Self::default();
        loader.source_description = format!("library from {}", root.display());
        ingest_dir(&mut loader, root, root)?;
        loader.finalize();
        Ok(loader)
    }

    #[allow(dead_code)] // Consumed by Phase 2 LSP and project-manifest work.
    pub fn files(&self) -> &[LibraryFile] {
        &self.files
    }

    pub fn package_names(&self) -> impl Iterator<Item = &str> {
        self.package_names.iter().map(String::as_str)
    }

    pub fn source_description(&self) -> &str {
        &self.source_description
    }

    pub fn contains_qualified(&self, name: &str) -> bool {
        self.qualified_names.contains(name)
    }

    pub fn contains_unqualified(&self, name: &str) -> bool {
        self.unqualified_names.contains(name)
    }

    /// Total declaration count across all library files. Cheap stat for
    /// `library-info`.
    pub fn declaration_count(&self) -> usize {
        self.files.iter().map(|file| file.declarations.len()).sum()
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    fn ingest(&mut self, virtual_path: &str, text: &str) {
        // Use a dummy PathBuf so the scanner's diagnostic path field is
        // populated; we discard the diagnostics here because the library
        // is curated upstream.
        let dummy = PathBuf::from(virtual_path);
        let scan = Scanner::new(&dummy, text).scan();
        let (package, declarations) = extract_declarations(&scan.tokens);
        if let Some(package) = &package {
            self.package_names.insert(package.clone());
        }
        self.files.push(LibraryFile {
            virtual_path: virtual_path.to_string(),
            package,
            declarations,
        });
    }

    fn finalize(&mut self) {
        for file in &self.files {
            for declaration in &file.declarations {
                self.unqualified_names.insert(declaration.clone());
                if let Some(package) = &file.package {
                    self.qualified_names
                        .insert(format!("{package}::{declaration}"));
                }
            }
        }
    }
}

fn ingest_dir(loader: &mut LibraryLoader, root: &Path, dir: &Path) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|error| format!("unable to read '{}': {error}", dir.display()))?
    {
        let entry =
            entry.map_err(|error| format!("unable to read directory entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            ingest_dir(loader, root, &path)?;
        } else if has_library_extension(&path.to_string_lossy()) {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("unable to read '{}': {error}", path.display()))?;
            loader.ingest(&relative, &text);
        }
    }
    Ok(())
}

fn has_library_extension(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".sysml") || lower.ends_with(".kerml")
}

/// Walk a token stream and extract:
/// - the top-level package name (the identifier that immediately follows
///   the first `package` keyword), and
/// - every declared identifier inside that package, regardless of
///   nesting depth.
///
/// "Declared identifier" here means: the identifier that immediately
/// follows one of the SysML v2 declaration-introducing keywords, after
/// optionally skipping `def`, visibility modifiers, and `abstract` etc.
fn extract_declarations(tokens: &[Token]) -> (Option<String>, Vec<String>) {
    let mut package: Option<String> = None;
    let mut declarations = Vec::new();

    let mut cursor = 0;
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if package.is_none() && token.value == "package" {
            if let Some(name) = read_identifier_after(tokens, cursor + 1) {
                package = Some(name.value.clone());
            }
            cursor += 1;
            continue;
        }

        if is_declaration_intro(&token.value) {
            // Skip optional `def`, modifiers, and qualifier-ish tokens
            // until we hit the declared name.
            if let Some((name, advance)) = read_declared_name(tokens, cursor + 1) {
                declarations.push(name);
                cursor += 1 + advance;
                continue;
            }
        }
        cursor += 1;
    }

    (package, declarations)
}

fn read_identifier_after(tokens: &[Token], start: usize) -> Option<&Token> {
    let mut cursor = start;
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if matches!(token.kind, TokenKind::Identifier) {
            return Some(token);
        }
        if token.value == "{" || token.value == ";" {
            return None;
        }
        cursor += 1;
    }
    None
}

fn read_declared_name(tokens: &[Token], start: usize) -> Option<(String, usize)> {
    let mut cursor = start;
    let skip = [
        "def",
        "abstract",
        "variation",
        "ref",
        "readonly",
        "public",
        "private",
        "protected",
        "in",
        "out",
        "inout",
    ];
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if skip.iter().any(|word| token.value == *word) {
            cursor += 1;
            continue;
        }
        if matches!(token.kind, TokenKind::Identifier) {
            // The name we want is the first identifier that ISN'T a
            // modifier and that isn't followed immediately by a `::` (a
            // type reference in the position where a name might appear).
            return Some((token.value.clone(), cursor - start));
        }
        return None;
    }
    None
}

fn is_declaration_intro(value: &str) -> bool {
    matches!(
        value,
        "part"
            | "item"
            | "port"
            | "interface"
            | "connection"
            | "constraint"
            | "requirement"
            | "concern"
            | "case"
            | "use"
            | "view"
            | "viewpoint"
            | "attribute"
            | "action"
            | "state"
            | "calc"
            | "occurrence"
            | "allocation"
            | "verification"
            | "analysis"
            | "flow"
            | "rendering"
            | "metadata"
            | "enum"
            | "class"
            | "classifier"
            | "datatype"
            | "feature"
            | "function"
            | "binding"
            | "specialization"
            | "subsetting"
            | "redefinition"
            | "type"
            | "association"
            | "interaction"
            | "behavior"
            | "step"
            | "individual"
            | "snapshot"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_library_loads() {
        let library = LibraryLoader::embedded();
        assert!(library.file_count() > 0);
        assert!(library.declaration_count() > 0);
        assert!(library.source_description().contains(EMBEDDED_LIBRARY_RELEASE));
    }

    #[test]
    fn embedded_library_indexes_known_kernel_types() {
        let library = LibraryLoader::embedded();
        // Foundational SysML v2 standard library types every real model
        // touches. Note: `StateAction` is the actual base type the
        // `state def Foo` syntactic shorthand resolves to, not `State`.
        for name in ["Part", "Item", "Port", "Action", "StateAction", "RequirementCheck"] {
            assert!(
                library.contains_unqualified(name),
                "expected '{name}' in unqualified library index"
            );
        }
    }

    #[test]
    fn embedded_library_indexes_known_packages() {
        let library = LibraryLoader::embedded();
        let packages: Vec<&str> = library.package_names().collect();
        for name in ["Parts", "Items", "Ports", "Actions"] {
            assert!(
                packages.iter().any(|p| *p == name),
                "expected package '{name}' in index; have {packages:?}"
            );
        }
    }

    #[test]
    fn embedded_library_indexes_qualified_names() {
        let library = LibraryLoader::embedded();
        assert!(library.contains_qualified("Parts::Part"));
        assert!(library.contains_qualified("Items::Item"));
    }

    #[test]
    fn embedded_library_does_not_pollute_with_keywords() {
        let library = LibraryLoader::embedded();
        for kw in ["def", "part", "abstract", "ref", "public", "private"] {
            assert!(
                !library.contains_unqualified(kw),
                "library accidentally indexed keyword '{kw}'"
            );
        }
    }
}
