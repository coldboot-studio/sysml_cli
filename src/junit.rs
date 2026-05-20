//! JUnit XML output (US-102).
//!
//! Maven Surefire / Jenkins JUnit format. One `<testsuite>` per validated
//! file. Each error diagnostic becomes a `<testcase>` with a `<failure>`.
//! Warnings become `<testcase>` with `<system-out>` annotation, or with
//! `<error>` when `--fail-on-warning` is set.

use std::fmt::Write as _;

use crate::diag::{Diagnostic, Severity, ValidationResult};

pub fn emit(results: &[ValidationResult], fail_on_warning: bool) -> String {
    let total_tests: usize = results
        .iter()
        .map(|result| {
            result
                .diagnostics
                .iter()
                .filter(|d| !d.is_suppressed())
                .count()
        })
        .sum();
    let total_failures: usize = results.iter().map(ValidationResult::error_count).sum();
    let total_warnings: usize = results.iter().map(ValidationResult::warning_count).sum();
    let total_errors_attr = if fail_on_warning { total_warnings } else { 0 };

    let mut output = String::new();
    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    writeln!(
        output,
        "<testsuites name=\"sysml-validate\" tests=\"{total_tests}\" failures=\"{total_failures}\" errors=\"{total_errors_attr}\">"
    )
    .expect("write to String");

    for result in results {
        let visible: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| !d.is_suppressed())
            .collect();
        let suite_failures = visible
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let suite_errors_attr = if fail_on_warning {
            visible
                .iter()
                .filter(|d| d.severity == Severity::Warning)
                .count()
        } else {
            0
        };
        let suite_path = result.path.to_string_lossy();
        writeln!(
            output,
            "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"{}\" skipped=\"0\" time=\"0\">",
            xml_attr_escape(&suite_path),
            visible.len(),
            suite_failures,
            suite_errors_attr,
        )
        .expect("write to String");

        for (index, diagnostic) in visible.iter().enumerate() {
            emit_testcase(&mut output, index, diagnostic, &suite_path, fail_on_warning);
        }
        output.push_str("  </testsuite>\n");
    }

    output.push_str("</testsuites>\n");
    output
}

fn emit_testcase(
    output: &mut String,
    index: usize,
    diagnostic: &Diagnostic,
    suite_path: &str,
    fail_on_warning: bool,
) {
    let location = diagnostic
        .position
        .as_ref()
        .map(|position| format!(":{}:{}", position.line, position.column))
        .unwrap_or_default();
    let name = format!("{}{} ({})", diagnostic.code, location, index);

    writeln!(
        output,
        "    <testcase classname=\"{}\" name=\"{}\" time=\"0\">",
        xml_attr_escape(suite_path),
        xml_attr_escape(&name),
    )
    .expect("write to String");

    match diagnostic.severity {
        Severity::Error => emit_failure(output, diagnostic),
        Severity::Warning if fail_on_warning => emit_error(output, diagnostic),
        Severity::Warning => emit_system_out(output, diagnostic),
        Severity::Info => emit_system_out(output, diagnostic),
    }

    output.push_str("    </testcase>\n");
}

fn emit_failure(output: &mut String, diagnostic: &Diagnostic) {
    writeln!(
        output,
        "      <failure type=\"{}\" message=\"{}\">{}</failure>",
        xml_attr_escape(diagnostic.code),
        xml_attr_escape(&diagnostic.message),
        xml_text_escape(&diagnostic.message),
    )
    .expect("write to String");
}

fn emit_error(output: &mut String, diagnostic: &Diagnostic) {
    writeln!(
        output,
        "      <error type=\"{}\" message=\"{}\">{}</error>",
        xml_attr_escape(diagnostic.code),
        xml_attr_escape(&diagnostic.message),
        xml_text_escape(&diagnostic.message),
    )
    .expect("write to String");
}

fn emit_system_out(output: &mut String, diagnostic: &Diagnostic) {
    writeln!(
        output,
        "      <system-out>{} {}: {}</system-out>",
        diagnostic.severity.as_str().to_ascii_uppercase(),
        diagnostic.code,
        xml_text_escape(&diagnostic.message),
    )
    .expect("write to String");
}

fn xml_attr_escape(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            '\n' => output.push_str("&#10;"),
            '\r' => output.push_str("&#13;"),
            '\t' => output.push_str("&#9;"),
            character => output.push(character),
        }
    }
    output
}

fn xml_text_escape(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            character => output.push(character),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::{Diagnostic, Position, Severity};
    use std::path::PathBuf;

    fn make_result_with_diagnostics(diagnostics: Vec<Diagnostic>) -> ValidationResult {
        let path = PathBuf::from("/project/model.sysml");
        let mut result = ValidationResult::new(&path);
        result.diagnostics = diagnostics;
        result
    }

    #[test]
    fn emits_failure_for_error() {
        let path = PathBuf::from("/project/model.sysml");
        let diagnostic = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name 'Engine'.",
            &path,
            Some(Position { line: 5, column: 1 }),
        );
        let result = make_result_with_diagnostics(vec![diagnostic]);
        let output = emit(&[result], false);
        assert!(output.contains("<failure type=\"SYSML041\""));
        assert!(output.contains("failures=\"1\""));
    }

    #[test]
    fn emits_system_out_for_warning_without_fail_on_warning() {
        let path = PathBuf::from("/project/model.sysml");
        let diagnostic = Diagnostic::new(
            Severity::Warning,
            "SYSML040",
            "Unresolved reference.",
            &path,
            Some(Position { line: 5, column: 1 }),
        );
        let result = make_result_with_diagnostics(vec![diagnostic]);
        let output = emit(&[result], false);
        assert!(output.contains("<system-out>"));
        assert!(!output.contains("<failure"));
        assert!(!output.contains("<error"));
    }

    #[test]
    fn promotes_warning_to_error_under_fail_on_warning() {
        let path = PathBuf::from("/project/model.sysml");
        let diagnostic = Diagnostic::new(
            Severity::Warning,
            "SYSML040",
            "Unresolved reference.",
            &path,
            Some(Position { line: 5, column: 1 }),
        );
        let result = make_result_with_diagnostics(vec![diagnostic]);
        let output = emit(&[result], true);
        assert!(output.contains("<error type=\"SYSML040\""));
        assert!(output.contains("errors=\"1\""));
    }

    #[test]
    fn excludes_suppressed_diagnostics() {
        let path = PathBuf::from("/project/model.sysml");
        let mut diagnostic = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name.",
            &path,
            Some(Position { line: 5, column: 1 }),
        );
        diagnostic.suppression = Some("Suppressed.".into());
        let result = make_result_with_diagnostics(vec![diagnostic]);
        let output = emit(&[result], false);
        assert!(output.contains("tests=\"0\""));
        assert!(!output.contains("<failure"));
    }

    #[test]
    fn escapes_xml_special_characters() {
        let path = PathBuf::from("/project/model.sysml");
        let diagnostic = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Bad <thing> & \"stuff\".",
            &path,
            None,
        );
        let result = make_result_with_diagnostics(vec![diagnostic]);
        let output = emit(&[result], false);
        assert!(output.contains("&lt;thing&gt;"));
        assert!(output.contains("&amp;"));
        assert!(output.contains("&quot;stuff&quot;"));
    }
}
