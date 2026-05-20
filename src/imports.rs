//! Import-clause parsing (US-203 first half).
//!
//! Grammar per KerML §8.2.3.4.2:
//!
//! ```text
//! Import = VisibilityIndicator? 'import' 'all'? ImportDeclaration RelationshipBody
//! ImportDeclaration = MembershipImport | NamespaceImport
//! MembershipImport = QualifiedName ( '::' '**' )?
//! NamespaceImport = QualifiedName '::' '*' ( '::' '**' )?
//! VisibilityIndicator = 'public' | 'private' | 'protected'
//! RelationshipBody = ';' | '{' ... '}'
//! ```
//!
//! This module walks a token stream and produces a `Vec<ParsedImport>`. It
//! does NOT perform the resolution itself — that lives in
//! [`crate::project`] (cross-file index) and in
//! [`crate::validate::validate_reference_candidates`] (per-call checks).

use crate::lex::{Token, TokenKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportShape {
    /// `import A::B::C;` — the single membership `C` is brought into scope.
    Membership { recursive: bool },
    /// `import A::B::*;` — all direct members of `A::B` are brought into
    /// scope. `recursive` is true for `A::B::**` (transitive members).
    Namespace { recursive: bool },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedImport {
    pub visibility: Visibility,
    pub import_all: bool,
    pub qualified_path: Vec<String>,
    pub shape: ImportShape,
}

impl ParsedImport {
    /// The leaf identifier brought into scope by a membership import (the
    /// last segment of the qualified path). For namespace imports, returns
    /// `None` — the leaf is wildcard.
    pub fn membership_leaf(&self) -> Option<&str> {
        match self.shape {
            ImportShape::Membership { .. } => self.qualified_path.last().map(String::as_str),
            ImportShape::Namespace { .. } => None,
        }
    }

    /// The namespace whose direct members are brought into scope by a
    /// namespace import (everything before the `::*`). For membership
    /// imports, returns `None`.
    pub fn namespace_root(&self) -> Option<String> {
        match self.shape {
            ImportShape::Namespace { .. } => Some(self.qualified_path.join("::")),
            ImportShape::Membership { .. } => None,
        }
    }

    /// Fully qualified path of a membership import (e.g., `Parts::Part`).
    /// For namespace imports, returns the namespace path without the `::*`.
    #[allow(dead_code)] // Consumed by Phase 2+ LSP and ported Pilot rules.
    pub fn qualified_string(&self) -> String {
        self.qualified_path.join("::")
    }
}

pub fn extract_imports(tokens: &[Token]) -> Vec<ParsedImport> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while cursor < tokens.len() {
        // Visibility prefix is optional and precedes `import`.
        let (visibility, advance) = read_optional_visibility(tokens, cursor);
        let import_index = cursor + advance;
        if tokens.get(import_index).map(|token| token.value.as_str()) != Some("import") {
            cursor += 1;
            continue;
        }

        let mut walker = import_index + 1;
        let mut import_all = false;
        if tokens.get(walker).map(|t| t.value.as_str()) == Some("all") {
            import_all = true;
            walker += 1;
        }

        let parsed = match parse_import_declaration(tokens, walker) {
            Some((parsed, consumed)) => {
                walker = consumed;
                ParsedImport {
                    visibility,
                    import_all,
                    qualified_path: parsed.qualified_path,
                    shape: parsed.shape,
                }
            }
            None => {
                // Malformed import; skip past it conservatively so we don't
                // double-recognize the same `import` keyword again.
                cursor = import_index + 1;
                continue;
            }
        };
        out.push(parsed);

        // Skip the relationship body (`;` or balanced `{ ... }`) so we don't
        // accidentally treat tokens inside a body as a new import.
        cursor = skip_relationship_body(tokens, walker);
    }
    out
}

fn read_optional_visibility(tokens: &[Token], start: usize) -> (Visibility, usize) {
    let visibility = tokens
        .get(start)
        .and_then(|token| match token.value.as_str() {
            "public" => Some(Visibility::Public),
            "private" => Some(Visibility::Private),
            "protected" => Some(Visibility::Protected),
            _ => None,
        });
    match visibility {
        Some(v) => (v, 1),
        // KerML default visibility on an import without explicit marker is
        // private per §8.2.3.4.2. We track it as Private for downstream
        // semantic enforcement that lands with US-205+.
        None => (Visibility::Private, 0),
    }
}

fn parse_import_declaration(tokens: &[Token], start: usize) -> Option<(ParsedImport, usize)> {
    let mut cursor = start;
    let mut path = Vec::new();
    let mut namespace_wildcard = false;
    let mut recursive = false;

    loop {
        let token = tokens.get(cursor)?;
        match token.kind {
            TokenKind::Identifier => {
                path.push(token.value.clone());
                cursor += 1;
            }
            // 'import Base::Anything' — keywords can appear in qualified
            // names if they are quoted, but we treat any identifier-like
            // token here. Bail otherwise.
            _ => return None,
        }
        match tokens.get(cursor).map(|t| t.value.as_str()) {
            Some("::") => {
                cursor += 1;
                match tokens.get(cursor).map(|t| t.value.as_str()) {
                    Some("*") => {
                        namespace_wildcard = true;
                        cursor += 1;
                        // Optional `::**` for recursive namespace import.
                        if tokens.get(cursor).map(|t| t.value.as_str()) == Some("::")
                            && tokens.get(cursor + 1).map(|t| t.value.as_str()) == Some("**")
                        {
                            recursive = true;
                            cursor += 2;
                        }
                        break;
                    }
                    Some("**") => {
                        recursive = true;
                        cursor += 1;
                        break;
                    }
                    _ => continue, // next identifier in the qualified path
                }
            }
            _ => break,
        }
    }

    if path.is_empty() {
        return None;
    }

    let shape = if namespace_wildcard {
        ImportShape::Namespace { recursive }
    } else {
        ImportShape::Membership { recursive }
    };

    Some((
        ParsedImport {
            visibility: Visibility::Private, // overwritten by caller
            import_all: false,               // overwritten by caller
            qualified_path: path,
            shape,
        },
        cursor,
    ))
}

fn skip_relationship_body(tokens: &[Token], start: usize) -> usize {
    let mut cursor = start;
    let mut depth = 0usize;
    while cursor < tokens.len() {
        match tokens[cursor].value.as_str() {
            ";" if depth == 0 => return cursor + 1,
            "{" => depth += 1,
            "}" if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    return cursor + 1;
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    cursor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::Scanner;
    use std::path::Path;

    fn parse(source: &str) -> Vec<ParsedImport> {
        let scan = Scanner::new(Path::new("test"), source).scan();
        extract_imports(&scan.tokens)
    }

    #[test]
    fn parses_simple_membership_import() {
        let imports = parse("package P { import Parts::Part; }");
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].qualified_string(), "Parts::Part");
        assert_eq!(imports[0].membership_leaf(), Some("Part"));
        assert_eq!(imports[0].visibility, Visibility::Private);
        assert_eq!(
            imports[0].shape,
            ImportShape::Membership { recursive: false }
        );
    }

    #[test]
    fn parses_visibility_prefix() {
        let imports = parse("private import Foo::Bar; public import Baz::Quux;");
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].visibility, Visibility::Private);
        assert_eq!(imports[1].visibility, Visibility::Public);
    }

    #[test]
    fn parses_namespace_wildcard() {
        let imports = parse("import Parts::*;");
        assert_eq!(imports.len(), 1);
        assert_eq!(
            imports[0].shape,
            ImportShape::Namespace { recursive: false }
        );
        assert_eq!(imports[0].namespace_root(), Some("Parts".into()));
        assert!(imports[0].membership_leaf().is_none());
    }

    #[test]
    fn parses_recursive_namespace_wildcard() {
        let imports = parse("import Parts::**;");
        assert_eq!(imports.len(), 1);
        assert_eq!(
            imports[0].shape,
            ImportShape::Membership { recursive: true }
        );
        assert_eq!(imports[0].qualified_string(), "Parts");
    }

    #[test]
    fn parses_recursive_namespace_star_double() {
        let imports = parse("import Parts::*::**;");
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].shape, ImportShape::Namespace { recursive: true });
    }

    #[test]
    fn parses_import_all() {
        let imports = parse("import all Parts::Part;");
        assert_eq!(imports.len(), 1);
        assert!(imports[0].import_all);
    }

    #[test]
    fn parses_multi_segment_qualified_name() {
        let imports = parse("import A::B::C::D;");
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].qualified_path, vec!["A", "B", "C", "D"]);
        assert_eq!(imports[0].membership_leaf(), Some("D"));
    }

    #[test]
    fn extracts_multiple_imports_in_package() {
        let imports = parse(
            "package P {\n  private import Foo::Bar;\n  import Baz::Quux;\n  import Items::*;\n}",
        );
        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].visibility, Visibility::Private);
        assert_eq!(imports[1].qualified_string(), "Baz::Quux");
        assert_eq!(
            imports[2].shape,
            ImportShape::Namespace { recursive: false }
        );
    }

    #[test]
    fn does_not_misparse_use_of_import_keyword_inside_string() {
        // String literals are tokenized as TokenKind::String, not as the
        // keyword 'import', so this should still find exactly one import.
        let imports = parse("package P { import Foo::Bar; doc /* import faked */ }");
        assert_eq!(imports.len(), 1);
    }
}
