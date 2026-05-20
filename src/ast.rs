//! Tree-sitter-backed parser (US-201 / Batch K).
//!
//! Wraps the `tree-sitter-sysml` grammar in a thin API focused on the
//! queries our validators care about today:
//!
//! - declared-name extraction with **proper scope context** (closes the
//!   false-positive for metadata-tag-introduced declarations like
//!   `#system foo { ... }` that our token-walker doesn't recognize)
//! - parent-type chain for redefinition / specialization (lets us tell
//!   that `port po1 :>> po1` is redefining the inherited member rather
//!   than self-referencing — Batch L will close out SYSML213's
//!   inherited-member ambiguity using this)
//!
//! The token-based validators in [`crate::validate`] continue to fire
//! for everything else; this module is additive. As more checks
//! migrate to AST-aware implementations the token walkers shrink.

use std::collections::HashSet;
use std::path::Path;

use tree_sitter::{Node, Parser, Tree};

use crate::diag::{Diagnostic, Position, Severity};

/// Parse a single source file. Returns `None` if the parser is
/// unavailable or the input cannot be parsed at all. Even with
/// syntactically invalid input, tree-sitter normally produces a tree
/// containing ERROR nodes; we surface those as `SYSML100` diagnostics.
pub struct ParseResult {
    pub tree: Tree,
    pub source: String,
}

pub fn parse(source: &str) -> Option<ParseResult> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_sysml::LANGUAGE.into())
        .ok()?;
    let tree = parser.parse(source, None)?;
    Some(ParseResult {
        tree,
        source: source.to_string(),
    })
}

/// SYSML100 — report tree-sitter's ERROR / MISSING nodes as parser
/// diagnostics. The token-level checks already cover many of these
/// (unterminated string, balanced delimiters), but tree-sitter catches
/// shapes those don't, like `part def {` with no name.
///
/// We deliberately emit only one diagnostic per ERROR node, not one
/// per child, to keep output manageable.
pub fn parse_diagnostics(result: &ParseResult, path: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let root = result.tree.root_node();
    walk_for_errors(root, &result.source, path, &mut diagnostics);
    diagnostics
}

fn walk_for_errors(node: Node<'_>, source: &str, path: &Path, out: &mut Vec<Diagnostic>) {
    if node.is_error() {
        let span = node_range_text(node, source);
        out.push(Diagnostic::new(
            Severity::Warning,
            "SYSML100",
            format!("Parser could not understand this region: {span}"),
            path,
            Some(node_position(node)),
        ));
        return; // don't recurse into ERROR subtree — too noisy
    }
    if node.is_missing() {
        out.push(Diagnostic::new(
            Severity::Warning,
            "SYSML100",
            format!(
                "Parser expected a `{}` here but did not find one.",
                node.kind()
            ),
            path,
            Some(node_position(node)),
        ));
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_errors(child, source, path, out);
    }
}

fn node_position(node: Node<'_>) -> Position {
    let start = node.start_position();
    Position {
        line: start.row + 1,
        column: start.column + 1,
    }
}

fn node_range_text(node: Node<'_>, source: &str) -> String {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let truncated = text.chars().take(40).collect::<String>();
    if text.chars().count() > 40 {
        format!("'{truncated}...'")
    } else {
        format!("'{truncated}'")
    }
}

/// Walk the AST and collect every identifier that the grammar
/// classifies as a declared name. This includes metadata-tag-introduced
/// declarations our token recognizer was blind to.
///
/// Returns a `HashSet<String>` of unqualified names, suitable for use
/// alongside the existing `declared_in_file` set in
/// [`crate::validate`].
pub fn collect_declared_names(result: &ParseResult) -> HashSet<String> {
    let mut names = HashSet::new();
    let root = result.tree.root_node();
    walk_for_declarations(root, &result.source, &mut names);
    names
}

fn walk_for_declarations(node: Node<'_>, source: &str, names: &mut HashSet<String>) {
    // The tree-sitter-sysml grammar nests declared names under an
    // `identification` node: `(identification (name (identifier)))`.
    // Walk all descendants and pull every `identifier` reached via that
    // path. This catches `part def Foo`, `attribute bar`, `package P`,
    // metadata-tag-introduced declarations, and any other shape where
    // the grammar recognized the construct as having an `identification`.
    if node.kind() == "identification" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "name" {
                let mut name_cursor = child.walk();
                for grandchild in child.children(&mut name_cursor) {
                    if grandchild.kind() == "identifier" {
                        if let Ok(text) = grandchild.utf8_text(source.as_bytes()) {
                            let trimmed = text.trim_matches('\'').to_string();
                            if !trimmed.is_empty() {
                                names.insert(trimmed);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_declarations(child, source, names);
    }
}

/// Inclusive line range, 1-based, matching `Position::line`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    pub fn contains(self, line: usize) -> bool {
        line >= self.start && line <= self.end
    }
}

/// Collect line-ranges of `usage_body` regions whose enclosing
/// `*_usage` declares a parent type via `typing_part`. Inside these
/// regions, `feature x :>> x` (or `:> x`) is overwhelmingly the
/// legitimate "redefine the inherited member" pattern rather than a
/// real self-reference bug. The structural validator consults this
/// list to suppress SYSML212/213 inside such regions.
pub fn inherited_member_redefinition_zones(result: &ParseResult) -> Vec<LineRange> {
    let mut zones = Vec::new();
    let root = result.tree.root_node();
    walk_for_inherited_zones(root, &mut zones);
    zones
}

fn walk_for_inherited_zones(node: Node<'_>, zones: &mut Vec<LineRange>) {
    // The relevant pattern is `(*_usage (usage_declaration ... typing_part)
    // (usage_body ...))`. We look at any node whose kind ends with `_usage`
    // and check whether its declaration has a typing_part. If yes, every
    // descendant inside its usage_body is in an "inherited zone."
    let kind = node.kind();
    if kind.ends_with("_usage") {
        let mut cursor = node.walk();
        let mut has_typing = false;
        let mut body: Option<Node<'_>> = None;
        for child in node.children(&mut cursor) {
            match child.kind() {
                "usage_declaration" => {
                    let mut decl_cursor = child.walk();
                    for grand in child.children(&mut decl_cursor) {
                        if grand.kind() == "typing_part" {
                            has_typing = true;
                        }
                    }
                }
                "usage_body" => body = Some(child),
                _ => {}
            }
        }
        if has_typing {
            if let Some(body) = body {
                let start = body.start_position().row + 1;
                let end = body.end_position().row + 1;
                zones.push(LineRange { start, end });
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_inherited_zones(child, zones);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_package() {
        let result = parse("package P { part def Engine; }").expect("parser available");
        let root = result.tree.root_node();
        assert!(!root.has_error(), "tree has errors: {:?}", root.to_sexp());
    }

    #[test]
    fn collects_part_def_name() {
        let result = parse("package P { part def Engine; }").unwrap();
        let names = collect_declared_names(&result);
        assert!(names.contains("Engine"), "names: {names:?}");
    }

    #[test]
    fn collects_multiple_declarations() {
        let result =
            parse("package P { part def Engine; attribute mass; part def Wheel { part hub; } }")
                .unwrap();
        let names = collect_declared_names(&result);
        for expected in ["Engine", "mass", "Wheel", "hub", "P"] {
            assert!(names.contains(expected), "missing {expected}: {names:?}");
        }
    }

    #[test]
    fn collects_inherited_zone_for_typed_part() {
        let source = "package P { part def P1 { port po; } part p1 : P1 { port po :>> po; } }";
        let result = parse(source).unwrap();
        let zones = inherited_member_redefinition_zones(&result);
        assert!(
            !zones.is_empty(),
            "expected at least one inherited zone; got: {zones:?}"
        );
        // The redefinition `:>> po` is on the same line as `part p1 : P1`.
        // The zone is the usage_body of `part p1 : P1 { ... }`.
        assert!(zones.iter().any(|z| z.contains(1)));
    }

    #[test]
    fn no_inherited_zone_when_part_has_no_parent_type() {
        let source = "package P { part def Standalone { port po; } }";
        let result = parse(source).unwrap();
        let zones = inherited_member_redefinition_zones(&result);
        assert!(zones.is_empty(), "expected no zones; got: {zones:?}");
    }

    #[test]
    fn reports_parse_error_via_sysml100() {
        // `part def` with no name and no body should produce an ERROR.
        let result = parse("package P { part def }").unwrap();
        let diagnostics = parse_diagnostics(&result, std::path::Path::new("test"));
        assert!(
            diagnostics.iter().any(|d| d.code == "SYSML100"),
            "expected SYSML100; got: {diagnostics:?}"
        );
    }
}
