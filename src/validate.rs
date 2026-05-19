use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use crate::config::{Config, RuleOverride};
use crate::diag::{Diagnostic, Severity, ValidationResult};
use crate::lex::{Scanner, Token, TokenKind};
use crate::library::LibraryLoader;
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

    let scan = Scanner::new(path, &text).scan();
    let tokens = scan.tokens;
    let mut suppressions = scan.suppressions;
    let non_blank_lines = scan.non_blank_lines;

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
        result
            .diagnostics
            .extend(validate_reference_candidates(path, &tokens, library));
    }

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
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let declared: HashSet<String> = tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.kind == TokenKind::Identifier && is_declaration_name(tokens, *index)
        })
        .map(|(_, token)| token.value.clone())
        .collect();
    let reference_markers = [
        "for",
        "to",
        "from",
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

        // Resolve against the standard library. If the candidate is the
        // leaf of a qualified name (`A::B::Foo`), check the qualified
        // path; otherwise fall back to the unqualified name index.
        let qualified = read_qualified_name(tokens, index + 1);
        let resolved = if qualified.contains("::") {
            library.contains_qualified(&qualified)
        } else {
            library.contains_unqualified(&candidate.value)
        };
        if resolved {
            continue;
        }

        diagnostics.push(Diagnostic::new(
            Severity::Warning,
            "SYSML040",
            format!(
                "Reference '{}' is not declared in this file and was not found in the loaded library. It may resolve from a project import.",
                if qualified.contains("::") { &qualified } else { &candidate.value }
            ),
            path,
            Some(candidate.position()),
        ));
    }
    diagnostics
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
        let result = validate_native(&path, strict, &Config::default(), &library);
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
        config
            .rules
            .insert("SYSML040".into(), "error".into());
        let library = LibraryLoader::embedded();
        let result = validate_native(&path, true, &config, &library);
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
        let result = validate_native(&path, false, &config, &library);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "SYSML041"),
            "SYSML041 should be silenced; got: {:?}",
            result.diagnostics
        );
    }
}
