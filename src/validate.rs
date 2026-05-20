use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use crate::ast;
use crate::config::{Config, RuleOverride};
use crate::diag::{Diagnostic, Severity, ValidationResult};
use crate::imports::{extract_imports, ParsedImport};
use crate::lex::{Scanner, Token, TokenKind};
use crate::library::LibraryLoader;
use crate::project::ProjectIndex;
use crate::suppress::apply_suppressions;

pub fn is_supported_model_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str).map(str::to_ascii_lowercase),
        Some(extension) if extension == "sysml" || extension == "kerml"
    )
}

pub fn validate_native(
    path: &Path,
    strict: bool,
    config: &Config,
    library: &LibraryLoader,
    project: &ProjectIndex,
) -> ValidationResult {
    let mut result = ValidationResult::new(path);
    if !is_supported_model_path(path) {
        result.diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML010",
            "Unsupported file extension. Expected .sysml or .kerml.",
            path,
            None,
        ));
        return result;
    }

    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            result.diagnostics.push(Diagnostic::new(
                Severity::Error,
                "SYSML012",
                format!("Unable to read file as UTF-8 text: {error}"),
                path,
                None,
            ));
            return result;
        }
    };

    validate_text_into(path, &text, strict, config, library, project, result)
}

/// Validate an in-memory buffer. Used by the LSP server (US-206) where
/// the editor's current text may not yet be on disk. Skips the
/// SYSML010 extension check and the SYSML012 I/O error — the caller is
/// expected to have populated `text` itself.
pub fn validate_text(
    path: &Path,
    text: &str,
    strict: bool,
    config: &Config,
    library: &LibraryLoader,
    project: &ProjectIndex,
) -> ValidationResult {
    validate_text_into(
        path,
        text,
        strict,
        config,
        library,
        project,
        ValidationResult::new(path),
    )
}

fn validate_text_into(
    path: &Path,
    text: &str,
    strict: bool,
    config: &Config,
    library: &LibraryLoader,
    project: &ProjectIndex,
    mut result: ValidationResult,
) -> ValidationResult {
    let scan = Scanner::new(path, text).scan();
    let tokens = scan.tokens;
    let mut suppressions = scan.suppressions;
    let non_blank_lines = scan.non_blank_lines;

    // AST parse (US-201, Batch K). Surfaces metadata-tag declarations
    // and other shapes the token recognizer misses. Falls back silently
    // if the parser is unavailable — the token-based passes still run.
    let ast_parse = ast::parse(text);
    let ast_declared_names = ast_parse
        .as_ref()
        .map(ast::collect_declared_names)
        .unwrap_or_default();
    // SYSML100 emission gated behind `--strict` (Batch L). The
    // tree-sitter-sysml 0.1 grammar is still early-stage and produces
    // many ERROR nodes on perfectly valid models that exercise mature
    // SysML v2 features its grammar doesn't yet cover (171 warnings
    // on the scamp reference model alone). Surfacing those by default
    // would drown real diagnostics. Strict mode opts in.
    if strict {
        if let Some(parse) = ast_parse.as_ref() {
            result
                .diagnostics
                .extend(ast::parse_diagnostics(parse, path));
        }
    }

    result.diagnostics.extend(scan.diagnostics);
    result
        .diagnostics
        .extend(validate_balanced_delimiters(path, &tokens));
    result
        .diagnostics
        .extend(validate_statement_shapes(path, &tokens));
    result
        .diagnostics
        .extend(validate_duplicate_scope_members(path, &tokens));
    if strict {
        let imports = extract_imports(&tokens);
        result.diagnostics.extend(validate_reference_candidates(
            path,
            &tokens,
            library,
            project,
            &imports,
            &ast_declared_names,
        ));
    }

    // Phase 2 Batch H: structural rules that fire regardless of --strict
    // because they catch errors, not heuristic warnings.
    let inherited_zones = ast_parse
        .as_ref()
        .map(ast::inherited_member_redefinition_zones)
        .unwrap_or_default();
    result.diagnostics.extend(validate_specialization_structure(
        path,
        &tokens,
        library,
        project,
        &ast_declared_names,
        &inherited_zones,
    ));

    // Apply suppressions first so an unused-suppression notice respects
    // diagnostics that the next pass would have promoted/demoted/dropped.
    // Suppressions mark in place; they do not remove from the list.
    let unused_suppressions = apply_suppressions(
        &mut result.diagnostics,
        &mut suppressions,
        &non_blank_lines,
        path,
    );
    result.diagnostics.extend(unused_suppressions);

    apply_rule_overrides(&mut result.diagnostics, config);
    result
}

fn apply_rule_overrides(diagnostics: &mut Vec<Diagnostic>, config: &Config) {
    let mut kept = Vec::with_capacity(diagnostics.len());
    let drained = std::mem::take(diagnostics);
    for mut diagnostic in drained {
        match config.rule_override(diagnostic.code) {
            None => kept.push(diagnostic),
            Some(RuleOverride::Off) => {
                // Drop the diagnostic entirely. (Config "off" silences a
                // rule globally; in-source suppressions only mark.)
            }
            Some(RuleOverride::Level(level)) => {
                diagnostic.severity = level;
                kept.push(diagnostic);
            }
        }
    }
    *diagnostics = kept;
}

fn validate_balanced_delimiters(path: &Path, tokens: &[Token]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut stack: Vec<&Token> = Vec::new();
    for token in tokens {
        if matches!(token.value.as_str(), "{" | "(" | "[") {
            stack.push(token);
        } else if matches!(token.value.as_str(), "}" | ")" | "]") {
            match stack.last() {
                Some(open) if expected_close(open.value.as_str()) == Some(token.value.as_str()) => {
                    stack.pop();
                }
                _ => diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "SYSML020",
                    format!("Unmatched closing delimiter '{}'.", token.value),
                    path,
                    Some(token.position()),
                )),
            }
        }
    }
    for token in stack.into_iter().rev() {
        diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML021",
            format!(
                "Unclosed delimiter '{}', expected '{}'.",
                token.value,
                expected_close(token.value.as_str()).unwrap_or("?")
            ),
            path,
            Some(token.position()),
        ));
    }
    diagnostics
}

fn validate_statement_shapes(path: &Path, tokens: &[Token]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.value == "package" {
            diagnostics.extend(expect_name_and_terminator(path, tokens, index, "package"));
        } else if token.value == "library" {
            if tokens.get(index + 1).map(|token| token.value.as_str()) != Some("package") {
                diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "SYSML030",
                    "Expected 'package' after 'library'.",
                    path,
                    Some(token.position()),
                ));
            }
        } else if token.value == "import" {
            diagnostics.extend(expect_terminator(path, tokens, index, "import declaration"));
        } else if token.value == "alias" {
            diagnostics.extend(expect_terminator(path, tokens, index, "alias declaration"));
            if !contains_before_end(tokens, index, "for") {
                diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "SYSML031",
                    "Alias declaration must include 'for'.",
                    path,
                    Some(token.position()),
                ));
            }
        } else if token.value == "dependency" {
            diagnostics.extend(expect_terminator(path, tokens, index, "dependency"));
            if !contains_before_end(tokens, index, "to") {
                diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "SYSML032",
                    "Dependency declaration must include a supplier after 'to'.",
                    path,
                    Some(token.position()),
                ));
            }
        } else if is_definition_keyword(&token.value)
            && tokens.get(index + 1).map(|token| token.value.as_str()) == Some("def")
        {
            diagnostics.extend(expect_name_and_terminator(
                path,
                tokens,
                index + 1,
                &format!("{} definition", token.value),
            ));
        } else if is_usage_keyword(&token.value) {
            if previous_value(tokens, index) == Some("def") {
                continue;
            }
            if matches!(previous_value(tokens, index), Some("assert")) {
                continue;
            }
            if is_connector_short_form(&token.value) {
                diagnostics.extend(expect_terminator(
                    path,
                    tokens,
                    index,
                    &format!("{} usage", token.value),
                ));
                continue;
            }
            match tokens.get(index + 1) {
                None => diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "SYSML033",
                    format!(
                        "Expected a declared name or specialization after '{}'.",
                        token.value
                    ),
                    path,
                    Some(token.position()),
                )),
                Some(next)
                    if next.value == "{" && is_anonymous_block_usage_keyword(&token.value) =>
                {
                    diagnostics.extend(expect_terminator(
                        path,
                        tokens,
                        index,
                        &format!("{} usage", token.value),
                    ))
                }
                Some(next) if next.value == ";" || next.value == "{" => {
                    diagnostics.push(Diagnostic::new(
                        Severity::Error,
                        "SYSML033",
                        format!(
                            "Expected a declared name or specialization after '{}'.",
                            token.value
                        ),
                        path,
                        Some(token.position()),
                    ))
                }
                Some(_) => diagnostics.extend(expect_terminator(
                    path,
                    tokens,
                    index,
                    &format!("{} usage", token.value),
                )),
            }
        }
    }
    diagnostics
}

fn validate_duplicate_scope_members(path: &Path, tokens: &[Token]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut scopes: Vec<HashSet<String>> = vec![HashSet::new()];
    let mut pending_name: Option<(String, Token)> = None;
    for (index, token) in tokens.iter().enumerate() {
        match token.value.as_str() {
            "{" => {
                scopes.push(HashSet::new());
                if let Some((name, name_token)) = pending_name.take() {
                    let scope_index = scopes.len().saturating_sub(2);
                    record_name(
                        path,
                        &mut scopes[scope_index],
                        &name,
                        &name_token,
                        &mut diagnostics,
                    );
                }
            }
            "}" => {
                if scopes.len() > 1 {
                    scopes.pop();
                }
            }
            ";" => {
                if let Some((name, name_token)) = pending_name.take() {
                    let last = scopes.len() - 1;
                    record_name(
                        path,
                        &mut scopes[last],
                        &name,
                        &name_token,
                        &mut diagnostics,
                    );
                }
            }
            _ if starts_named_member(tokens, index) => {
                if let Some(name) = declared_name_after(tokens, index) {
                    pending_name = Some((name.value.clone(), name.clone()));
                }
            }
            _ => {}
        }
    }
    diagnostics
}

fn validate_reference_candidates(
    path: &Path,
    tokens: &[Token],
    library: &LibraryLoader,
    project: &ProjectIndex,
    imports: &[ParsedImport],
    ast_declared: &HashSet<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut declared: HashSet<String> = tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.kind == TokenKind::Identifier && is_declaration_name(tokens, *index)
        })
        .map(|(_, token)| token.value.clone())
        .collect();
    // Union with the AST-collected names so metadata-tag declarations
    // and other shapes the token recognizer misses don't produce
    // spurious unresolved-reference warnings.
    declared.extend(ast_declared.iter().cloned());

    // Pre-compute the set of unqualified leaf names that are pulled into
    // scope by membership imports (e.g., `import Foo::Bar;` makes `Bar`
    // available) and the set of namespaces whose direct members are
    // wildcard-imported (`import Foo::*;` → `"Foo"`).
    let imported_leaves: HashSet<&str> = imports
        .iter()
        .filter_map(ParsedImport::membership_leaf)
        .collect();
    let wildcard_namespaces: Vec<String> = imports
        .iter()
        .filter_map(ParsedImport::namespace_root)
        .collect();

    // Reference markers — the token AFTER one of these is the
    // identifier we want to resolve against the project + library +
    // imports. ':' is the typed-usage colon (`part foo : Bar`) which is
    // by far the most common reference site in real SysML v2 models;
    // the longer `:>`, `:>>`, `:=`, `::` forms tokenize as distinct
    // operators and never reach a bare-`:` check.
    let reference_markers = [
        "for",
        "to",
        "from",
        ":",
        ":>",
        "specializes",
        "subsets",
        "references",
        "redefines",
    ];
    for (index, window) in tokens.windows(2).enumerate() {
        let marker = &window[0];
        let candidate = &window[1];
        if !reference_markers.contains(&marker.value.as_str())
            || candidate.kind != TokenKind::Identifier
        {
            continue;
        }
        if declared.contains(&candidate.value) {
            continue;
        }

        let qualified = read_qualified_name(tokens, index + 1);
        let is_qualified = qualified.contains("::");

        // Resolution order (US-203):
        //   1. Standard library (qualified or unqualified index).
        //   2. Project-wide symbol table (other files in the same run).
        //   3. Imports — explicit membership imports of the leaf name,
        //      or wildcard imports whose namespace contains the leaf.
        let resolved = if is_qualified {
            library.contains_qualified(&qualified) || project.contains_qualified(&qualified)
        } else {
            library.contains_unqualified(&candidate.value)
                || project.contains_unqualified(&candidate.value)
                || imported_leaves.contains(candidate.value.as_str())
                || wildcard_namespaces.iter().any(|namespace| {
                    library.contains_qualified(&format!("{namespace}::{}", candidate.value))
                        || project.namespace_contains(namespace, &candidate.value)
                })
        };
        if resolved {
            continue;
        }

        diagnostics.push(Diagnostic::new(
            Severity::Warning,
            "SYSML040",
            format!(
                "Reference '{}' is not declared in this file, not imported, and was not found in the project or standard library.",
                if is_qualified { &qualified } else { &candidate.value }
            ),
            path,
            Some(candidate.position()),
        ));
    }
    diagnostics
}

/// SYSML210/211/212/213 — structural rules over `:>` and `:>>` that
/// don't depend on `--strict`. These are errors, not warnings: the
/// model is provably malformed if any fire.
///
/// Walks the token stream once, looking for the patterns:
///   `<decl_name> :> <target>`     (specialization)
///   `<decl_name> :>> <target>`    (redefinition)
/// where `<decl_name>` is the most recently seen identifier preceding
/// the marker, and `<target>` is the (qualified) identifier that
/// follows it.
fn validate_specialization_structure(
    path: &Path,
    tokens: &[Token],
    library: &LibraryLoader,
    project: &ProjectIndex,
    ast_declared: &HashSet<String>,
    inherited_zones: &[ast::LineRange],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut declared_in_file = collect_declared_names(tokens);
    // Union with AST-collected names (Batch K). Stops SYSML210/211 from
    // false-firing on metadata-tag declarations like `#system foo {}`.
    declared_in_file.extend(ast_declared.iter().cloned());

    let mut cursor = 0;
    // last_decl is the most recently seen identifier WITHIN THE CURRENT
    // statement. It resets on `;` and `{` so identifiers from earlier
    // statements don't leak forward and cause cross-statement false
    // positives in the self-reference check.
    let mut last_decl: Option<&Token> = None;
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if matches!(token.value.as_str(), ";" | "{" | "}") {
            last_decl = None;
            cursor += 1;
            continue;
        }
        if token.kind == TokenKind::Identifier {
            last_decl = Some(token);
            cursor += 1;
            continue;
        }
        let is_specialization = token.value == ":>" || token.value == "specializes";
        let is_redefinition = token.value == ":>>" || token.value == "redefines";
        if !(is_specialization || is_redefinition) {
            cursor += 1;
            continue;
        }
        let Some(target_token) = tokens.get(cursor + 1) else {
            cursor += 1;
            continue;
        };
        if target_token.kind != TokenKind::Identifier {
            cursor += 1;
            continue;
        }
        let target_qualified = read_qualified_name(tokens, cursor + 1);
        let target_leaf = target_qualified
            .rsplit_once("::")
            .map(|(_, leaf)| leaf)
            .unwrap_or(target_qualified.as_str());

        // SYSML212 / SYSML213: self-reference.
        //
        // Important: only treat this as self-reference if the target is
        // UNqualified. `attribute mass :> ISQ::mass` declares a new
        // `mass` that specializes a differently-namespaced `ISQ::mass`;
        // those are not the same feature and must not fire SYSML212.
        // Same logic applies to redefinitions of library members.
        let target_is_qualified = target_qualified.contains("::");
        if !target_is_qualified {
            if let Some(decl) = last_decl {
                if decl.value == target_leaf {
                    // Batch L: AST-aware suppression. If the position is
                    // inside the usage_body of a typed `*_usage` (a part
                    // with a parent type), the matching same-name target
                    // is the inherited member of the parent, not a real
                    // self-reference. Skip the diagnostic entirely.
                    let position_line = target_token.line;
                    if inherited_zones
                        .iter()
                        .any(|zone| zone.contains(position_line))
                    {
                        cursor += 1;
                        continue;
                    }
                    let code: &'static str = if is_specialization {
                        "SYSML212"
                    } else {
                        "SYSML213"
                    };
                    let verb = if is_specialization {
                        "specialize"
                    } else {
                        "redefine"
                    };
                    diagnostics.push(Diagnostic::new(
                        Severity::Warning,
                        code,
                        format!(
                            "Feature '{}' appears to {verb} itself. This is often legitimate (redefinition of an inherited member with the same name) but is occasionally a typo. AST-aware checking will tighten this in a future release.",
                            decl.value
                        ),
                        path,
                        Some(target_token.position()),
                    ));
                    cursor += 1;
                    continue;
                }
            }
        }

        // SYSML210 / SYSML211: target does not resolve through any path.
        // Reuse the same resolution order as SYSML040 but promote to error.
        let resolves = if target_qualified.contains("::") {
            library.contains_qualified(&target_qualified)
                || project.contains_qualified(&target_qualified)
        } else {
            declared_in_file.contains(target_leaf)
                || library.contains_unqualified(target_leaf)
                || project.contains_unqualified(target_leaf)
        };
        if !resolves {
            let code: &'static str = if is_specialization {
                "SYSML210"
            } else {
                "SYSML211"
            };
            let kind = if is_specialization {
                "Specialization"
            } else {
                "Redefinition"
            };
            diagnostics.push(Diagnostic::new(
                Severity::Error,
                code,
                format!(
                    "{kind} target '{}' does not resolve. {kind} requires its target to exist.",
                    if target_qualified.contains("::") {
                        target_qualified.clone()
                    } else {
                        target_leaf.to_string()
                    }
                ),
                path,
                Some(target_token.position()),
            ));
        }

        cursor += 1;
    }

    diagnostics
}

fn collect_declared_names(tokens: &[Token]) -> HashSet<String> {
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.kind == TokenKind::Identifier && is_declaration_name(tokens, *index)
        })
        .map(|(_, token)| token.value.clone())
        .collect()
}

/// Read a qualified name starting at `start` (`Foo`, `A::B`, `A::B::C`,
/// etc.). Stops at the first token that isn't `::` or an identifier.
fn read_qualified_name(tokens: &[Token], start: usize) -> String {
    let mut parts = Vec::new();
    let mut cursor = start;
    loop {
        let Some(token) = tokens.get(cursor) else {
            break;
        };
        if token.kind != TokenKind::Identifier {
            break;
        }
        parts.push(token.value.as_str());
        cursor += 1;
        if tokens.get(cursor).map(|t| t.value.as_str()) != Some("::") {
            break;
        }
        cursor += 1;
    }
    parts.join("::")
}

fn record_name(
    path: &Path,
    scope: &mut HashSet<String>,
    name: &str,
    token: &Token,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if scope.contains(name) {
        diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML041",
            format!("Duplicate member name '{name}' in the same lexical scope."),
            path,
            Some(token.position()),
        ));
    }
    scope.insert(name.to_string());
}

fn expect_name_and_terminator(
    path: &Path,
    tokens: &[Token],
    index: usize,
    statement: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let name_index = index + 1;
    if tokens
        .get(name_index)
        .map(|token| token.value == ";" || token.value == "{")
        .unwrap_or(true)
    {
        diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML034",
            format!("Expected a name for {statement}."),
            path,
            tokens.get(index).map(Token::position),
        ));
    }
    diagnostics.extend(expect_terminator(path, tokens, index, statement));
    diagnostics
}

fn expect_terminator(
    path: &Path,
    tokens: &[Token],
    index: usize,
    statement: &str,
) -> Vec<Diagnostic> {
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut cursor = index + 1;
    while cursor < tokens.len() {
        match tokens[cursor].value.as_str() {
            "{" if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 => return Vec::new(),
            "{" => brace_depth += 1,
            "(" => paren_depth += 1,
            "[" => bracket_depth += 1,
            "}" if brace_depth > 0 => brace_depth -= 1,
            ")" if paren_depth > 0 => paren_depth -= 1,
            "]" if bracket_depth > 0 => bracket_depth -= 1,
            ";" if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 => return Vec::new(),
            _ => {}
        }
        cursor += 1;
    }
    vec![Diagnostic::new(
        Severity::Error,
        "SYSML035",
        format!("Expected ';' or '{{' to terminate {statement}."),
        path,
        tokens.get(index).map(Token::position),
    )]
}

fn starts_named_member(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token.value == "package"
        || (is_definition_keyword(&token.value) && next_value(tokens, index) == Some("def"))
        || (is_usage_keyword(&token.value)
            && !is_connector_short_form(&token.value)
            && previous_value(tokens, index) != Some("def")
            && !matches!(previous_value(tokens, index), Some("assert"))
            && !contains_before_end(tokens, index, ":>")
            && !contains_before_end(tokens, index, ":>>"))
}

fn declared_name_after(tokens: &[Token], index: usize) -> Option<&Token> {
    let mut cursor = index + 1;
    if tokens.get(cursor).map(|token| token.value.as_str()) == Some("def") {
        cursor += 1;
    }

    cursor = skip_metadata_prefix(tokens, cursor);
    let token = tokens.get(cursor)?;
    if is_unnamed_usage_prefix(&token.value) {
        return None;
    }
    if matches!(token.kind, TokenKind::Identifier | TokenKind::String) {
        if next_value(tokens, cursor) == Some(".") {
            return None;
        }
        return Some(token);
    }
    None
}

fn skip_metadata_prefix(tokens: &[Token], mut cursor: usize) -> usize {
    if tokens.get(cursor).map(|token| token.value.as_str()) != Some("<") {
        return cursor;
    }
    let mut depth = 0usize;
    while cursor < tokens.len() {
        match tokens[cursor].value.as_str() {
            "<" => depth += 1,
            ">" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return cursor + 1;
                }
            }
            ";" | "{" if depth == 0 => return cursor,
            _ => {}
        }
        cursor += 1;
    }
    cursor
}

fn is_unnamed_usage_prefix(value: &str) -> bool {
    matches!(
        value,
        ":" | ":>" | ":>>" | "::" | "." | "of" | "from" | "to" | "connect"
    )
}

fn is_declaration_name(tokens: &[Token], index: usize) -> bool {
    let start = index.saturating_sub(3);
    for cursor in start..index {
        if starts_named_member(tokens, cursor) {
            if let Some(name) = declared_name_after(tokens, cursor) {
                if name == &tokens[index] {
                    return true;
                }
            }
        }
    }
    false
}

fn contains_before_end(tokens: &[Token], index: usize, value: &str) -> bool {
    let mut cursor = index + 1;
    while cursor < tokens.len() && tokens[cursor].value != ";" && tokens[cursor].value != "{" {
        if tokens[cursor].value == value {
            return true;
        }
        cursor += 1;
    }
    false
}

fn expected_close(value: &str) -> Option<&'static str> {
    match value {
        "{" => Some("}"),
        "(" => Some(")"),
        "[" => Some("]"),
        _ => None,
    }
}

fn next_value(tokens: &[Token], index: usize) -> Option<&str> {
    tokens.get(index + 1).map(|token| token.value.as_str())
}

fn previous_value(tokens: &[Token], index: usize) -> Option<&str> {
    index
        .checked_sub(1)
        .and_then(|previous| tokens.get(previous))
        .map(|token| token.value.as_str())
}

fn is_connector_short_form(value: &str) -> bool {
    matches!(
        value,
        "bind" | "connect" | "satisfy" | "verify" | "include" | "perform" | "assert"
    )
}

fn is_anonymous_block_usage_keyword(value: &str) -> bool {
    matches!(value, "action" | "constraint")
}

fn is_definition_keyword(value: &str) -> bool {
    matches!(
        value,
        "attribute"
            | "allocation"
            | "analysis"
            | "action"
            | "calc"
            | "case"
            | "concern"
            | "connection"
            | "constraint"
            | "enum"
            | "flow"
            | "interface"
            | "item"
            | "occurrence"
            | "part"
            | "port"
            | "rendering"
            | "requirement"
            | "state"
            | "use"
            | "verification"
            | "view"
            | "viewpoint"
    )
}

fn is_usage_keyword(value: &str) -> bool {
    matches!(
        value,
        "action"
            | "allocation"
            | "assert"
            | "attribute"
            | "bind"
            | "case"
            | "concern"
            | "connect"
            | "connection"
            | "constraint"
            | "flow"
            | "include"
            | "interface"
            | "item"
            | "occurrence"
            | "part"
            | "perform"
            | "port"
            | "ref"
            | "requirement"
            | "satisfy"
            | "state"
            | "use"
            | "verify"
            | "view"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn accepts_basic_package() {
        let result = validate_temp(
            "package Vehicle { part def Engine; part engine : Engine; attribute mass; }",
            ".sysml",
            false,
        );
        assert!(result.ok(), "{:?}", result.diagnostics);
    }

    #[test]
    fn reports_unbalanced_delimiter() {
        let result = validate_temp("package Vehicle { part def Engine;", ".sysml", false);
        assert!(!result.ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SYSML021"));
    }

    #[test]
    fn reports_missing_alias_for() {
        let result = validate_temp("package P { alias E Engine; }", ".sysml", false);
        assert!(!result.ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SYSML031"));
    }

    #[test]
    fn reports_duplicate_member_in_scope() {
        let result = validate_temp(
            "package P { part def Engine; part def Engine; }",
            ".sysml",
            false,
        );
        assert!(!result.ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SYSML041"));
    }

    #[test]
    fn reports_strict_reference_warning() {
        let result = validate_temp("package P { part engine :> Missing; }", ".sysml", true);
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SYSML040"));
    }

    fn validate_temp(text: &str, suffix: &str, strict: bool) -> ValidationResult {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is before UNIX_EPOCH")
            .as_nanos();
        let dir = env::temp_dir().join(format!("sysml_validate_{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join(format!("model{suffix}"));
        fs::write(&path, text).expect("write temp model");
        let library = LibraryLoader::embedded();
        let project = ProjectIndex::empty();
        let result = validate_native(&path, strict, &Config::default(), &library, &project);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
        result
    }

    #[test]
    fn library_resolves_part_reference_in_strict_mode() {
        // Before US-202: `:> Part` produced a SYSML040 false positive
        // because `Part` is not declared in the user file. With the
        // embedded library loaded, `Part` resolves from `Parts::Part`.
        let result = validate_temp("package P { part engine :> Part; }", ".sysml", true);
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "SYSML040"),
            "library should resolve 'Part'; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn library_does_not_resolve_genuinely_missing_reference() {
        // Negative control: a name that is NOT in the library should
        // still warn under --strict.
        let result = validate_temp(
            "package P { part engine :> CompletelyMadeUpName123; }",
            ".sysml",
            true,
        );
        assert!(
            result.diagnostics.iter().any(|d| d.code == "SYSML040"),
            "missing name should warn; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn library_resolves_qualified_name() {
        let result = validate_temp("package P { part engine :> Parts::Part; }", ".sysml", true);
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "SYSML040"),
            "library should resolve 'Parts::Part'; got: {:?}",
            result.diagnostics
        );
    }

    // ---- US-203: import + cross-file resolution tests ----

    fn validate_with_project(
        text: &str,
        suffix: &str,
        strict: bool,
        project: &ProjectIndex,
    ) -> ValidationResult {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = env::temp_dir().join(format!("sysml_validate_xfile_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(format!("model{suffix}"));
        fs::write(&path, text).expect("write");
        let library = LibraryLoader::embedded();
        let result = validate_native(&path, strict, &Config::default(), &library, project);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
        result
    }

    #[test]
    fn explicit_membership_import_resolves_leaf() {
        let result = validate_temp(
            "package P { import Engines::Engine; part e :> Engine; }",
            ".sysml",
            true,
        );
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "SYSML040"),
            "import Engines::Engine should bring Engine into scope; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn wildcard_namespace_import_resolves_library_member() {
        // Parts::Part is in the embedded library; `import Parts::*;` should
        // make the bare name `Part` resolve.
        let result = validate_temp(
            "package P { import Parts::*; part e :> Part; }",
            ".sysml",
            true,
        );
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "SYSML040"),
            "wildcard library import should resolve 'Part'; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn project_index_resolves_cross_file_reference() {
        let project = ProjectIndex::from_tokens(
            Some("Engines".into()),
            vec!["Engine".into()],
            Path::new("/x"),
        );
        let result = validate_with_project(
            "package P { part e :> Engines::Engine; }",
            ".sysml",
            true,
            &project,
        );
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "SYSML040"),
            "project index should resolve 'Engines::Engine'; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn typed_usage_colon_catches_typo() {
        // Real-world bug: `part wheel : Whell;` is a typo for `Wheel`.
        // Pre-Batch-G.5 this passed silently because `:` was not in the
        // marker set; the gap surfaced when running against the scamp
        // reference model.
        let result = validate_temp(
            "package P { part def Wheel; part wheel : Whell; }",
            ".sysml",
            true,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "SYSML040" && d.message.contains("Whell")),
            "typed-usage typo 'Whell' should be flagged; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn typed_usage_colon_resolves_library_type() {
        // `attribute mass : Real;` is the most common typed-usage form;
        // `Real` lives in the embedded ScalarValues library and should
        // resolve cleanly. Negative case: `attribute mass : Bogus123;`
        // should still warn.
        let clean = validate_temp("package P { attribute mass : Real; }", ".sysml", true);
        assert!(
            !clean.diagnostics.iter().any(|d| d.code == "SYSML040"),
            "Real should resolve from library; got: {:?}",
            clean.diagnostics
        );
        let dirty = validate_temp("package P { attribute mass : Bogus123; }", ".sysml", true);
        assert!(
            dirty.diagnostics.iter().any(|d| d.code == "SYSML040"),
            "Bogus123 should warn; got: {:?}",
            dirty.diagnostics
        );
    }

    // ---- Batch H: structural rules (SYSML210/211/212/213/220) ----

    #[test]
    fn missing_specialization_target_is_error() {
        // Bare `:> Foo` where Foo doesn't exist anywhere. SYSML210 fires.
        let result = validate_temp(
            "package P { part def Engine :> NonexistentParent; }",
            ".sysml",
            false,
        );
        let sysml210: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "SYSML210")
            .collect();
        assert_eq!(
            sysml210.len(),
            1,
            "expected exactly one SYSML210; got: {:?}",
            result.diagnostics
        );
        assert_eq!(sysml210[0].severity, Severity::Error);
    }

    #[test]
    fn missing_redefinition_target_is_error() {
        let result = validate_temp(
            "package P { part def Engine { feature thrust :>> nonexistentFeature; } }",
            ".sysml",
            false,
        );
        assert!(
            result.diagnostics.iter().any(|d| d.code == "SYSML211"),
            "expected SYSML211; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn resolved_specialization_target_does_not_error() {
        // `:> Part` resolves via the embedded library; no SYSML210.
        let result = validate_temp("package P { part def Engine :> Part; }", ".sysml", false);
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "SYSML210"),
            "library-resolved target should not fire SYSML210; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn self_specialization_is_warning() {
        // Batch J: demoted from error to warning because the same token
        // pattern matches both real self-reference bugs AND the
        // legitimate redefinition-of-inherited-member case.
        let result = validate_temp("package P { part def Engine :> Engine; }", ".sysml", false);
        let hit = result
            .diagnostics
            .iter()
            .find(|d| d.code == "SYSML212")
            .expect("expected SYSML212");
        assert_eq!(hit.severity, Severity::Warning);
    }

    #[test]
    fn self_redefinition_is_warning() {
        let result = validate_temp(
            "package P { part def Engine { feature thrust :>> thrust; } }",
            ".sysml",
            false,
        );
        let hit = result
            .diagnostics
            .iter()
            .find(|d| d.code == "SYSML213")
            .expect("expected SYSML213");
        assert_eq!(hit.severity, Severity::Warning);
    }

    #[test]
    fn last_decl_resets_on_statement_boundary() {
        // Regression for Batch J: `attribute redefines Foo` declares no
        // new identifier, so the stale last_decl from an earlier
        // statement must not leak in and produce a false SYSML213.
        let result = validate_temp(
            "package P { part def Foo; attribute redefines Foo; }",
            ".sysml",
            false,
        );
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "SYSML213"),
            "stale last_decl leaked; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn import_does_not_resolve_unrelated_name() {
        // The import is for `Engine`; a reference to `Wheel` should still
        // warn (negative control).
        let result = validate_temp(
            "package P { import Engines::Engine; part w :> Wheel; }",
            ".sysml",
            true,
        );
        assert!(
            result.diagnostics.iter().any(|d| d.code == "SYSML040"),
            "Wheel should still warn; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn suppression_silences_duplicate_member() {
        let result = validate_temp(
            "package P { part def Engine; part def Engine; // sysml-validate: disable=SYSML041\n}",
            ".sysml",
            false,
        );
        // Batch C: suppressed diagnostics are KEPT in the list, marked
        // with .suppression. They are excluded from error_count and from
        // the exit-code decision (proven via result.ok()).
        let sysml041: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "SYSML041")
            .collect();
        assert_eq!(sysml041.len(), 1);
        assert!(sysml041[0].is_suppressed());
        assert!(result.ok(), "suppressed errors must not fail the run");
        assert_eq!(result.error_count(), 0);
        assert_eq!(result.suppressed_count(), 1);
    }

    #[test]
    fn unused_suppression_emits_sysml050() {
        let result = validate_temp(
            "package P { // sysml-validate: disable=SYSML041\n  part def Engine; }",
            ".sysml",
            false,
        );
        assert!(
            result.diagnostics.iter().any(|d| d.code == "SYSML050"),
            "expected SYSML050; got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn config_promotes_warning_to_error() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = env::temp_dir().join(format!("sysml_validate_promo_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("model.sysml");
        fs::write(&path, "package P { part engine :> Missing; }").expect("write");
        let mut config = Config::default();
        config.rules.insert("SYSML040".into(), "error".into());
        let library = LibraryLoader::embedded();
        let project = ProjectIndex::empty();
        let result = validate_native(&path, true, &config, &library, &project);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
        let promoted = result
            .diagnostics
            .iter()
            .find(|d| d.code == "SYSML040")
            .expect("expected SYSML040");
        assert_eq!(promoted.severity, Severity::Error);
    }

    #[test]
    fn config_off_drops_rule() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = env::temp_dir().join(format!("sysml_validate_off_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("model.sysml");
        fs::write(&path, "package P { part def Engine; part def Engine; }").expect("write");
        let mut config = Config::default();
        config.rules.insert("SYSML041".into(), "off".into());
        let library = LibraryLoader::embedded();
        let project = ProjectIndex::empty();
        let result = validate_native(&path, false, &config, &library, &project);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "SYSML041"),
            "SYSML041 should be silenced; got: {:?}",
            result.diagnostics
        );
    }
}
