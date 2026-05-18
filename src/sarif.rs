//! SARIF 2.1.0 output.
//!
//! Schema: <https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html>
//!
//! This emitter produces a single `runs[0]` per invocation. Suppressed
//! diagnostics are retained in `results[]` and marked with
//! `suppressions[].kind = "inSource"`, never silently dropped.

use std::path::Path;

use serde_json::{json, Value};

use crate::baseline::Baseline;
use crate::diag::{Diagnostic, Severity, ValidationResult};
use crate::report::RunMetadata;
use crate::rules::{Rule, CATALOG};

pub const SARIF_SCHEMA_URI: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/cos02/schemas/sarif-schema-2.1.0.json";
pub const SARIF_VERSION: &str = "2.1.0";

pub fn emit(
    results: &[ValidationResult],
    metadata: &RunMetadata,
    command_line: &str,
    exit_code: i32,
    end_timestamp_utc: &str,
    project_root: Option<&Path>,
    baseline: Option<&Baseline>,
) -> String {
    let rules: Vec<Value> = CATALOG.iter().map(rule_to_json).collect();
    let sarif_results: Vec<Value> = results
        .iter()
        .flat_map(|result| {
            result
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic_to_json(diagnostic, project_root, baseline))
        })
        .collect();

    let working_directory = std::env::current_dir()
        .ok()
        .map(|path| path_to_file_uri(&path))
        .unwrap_or_else(|| "file:///".to_string());

    let document = json!({
        "$schema": SARIF_SCHEMA_URI,
        "version": SARIF_VERSION,
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": metadata.tool_name,
                        "version": metadata.tool_version,
                        "semanticVersion": metadata.tool_version,
                        "informationUri": "https://github.com/Systems-Modeling/SysML-v2-Release",
                        "rules": rules,
                        "properties": {
                            "ruleCatalogVersion": metadata.rule_catalog_version,
                        },
                    }
                },
                "invocations": [
                    {
                        "commandLine": command_line,
                        "startTimeUtc": metadata.timestamp_utc,
                        "endTimeUtc": end_timestamp_utc,
                        "exitCode": exit_code,
                        "executionSuccessful": exit_code == 0,
                        "workingDirectory": { "uri": working_directory },
                    }
                ],
                "properties": {
                    "backend": metadata.backend,
                    "strict": metadata.strict,
                    "failOnWarning": metadata.fail_on_warning,
                },
                "results": sarif_results,
            }
        ]
    });

    serde_json::to_string_pretty(&document)
        .expect("SARIF JSON serialization cannot fail with serde_json::Value")
}

fn rule_to_json(rule: &Rule) -> Value {
    json!({
        "id": rule.id,
        "name": rule.name,
        "shortDescription": { "text": rule.short_description },
        "fullDescription": { "text": rule.full_description },
        "defaultConfiguration": { "level": sarif_level(rule.default_level) },
        "helpUri": rule.help_uri(),
    })
}

fn diagnostic_to_json(
    diagnostic: &Diagnostic,
    project_root: Option<&Path>,
    baseline: Option<&Baseline>,
) -> Value {
    let fingerprint = diagnostic.fingerprint(project_root);
    let mut value = json!({
        "ruleId": diagnostic.code,
        "level": sarif_level(diagnostic.severity),
        "message": { "text": diagnostic.message },
        "partialFingerprints": {
            "diagnosticHash/v1": fingerprint.clone(),
        },
    });

    let location = json!({
        "physicalLocation": physical_location(diagnostic),
    });
    value
        .as_object_mut()
        .expect("object")
        .insert("locations".to_string(), Value::Array(vec![location]));

    if let Some(reason) = &diagnostic.suppression {
        let suppression = json!([{
            "kind": "inSource",
            "justification": reason,
            "status": "accepted",
        }]);
        value
            .as_object_mut()
            .expect("object")
            .insert("suppressions".to_string(), suppression);
    }

    if let Some(baseline) = baseline {
        let state = baseline.classify(diagnostic.code, &fingerprint);
        value
            .as_object_mut()
            .expect("object")
            .insert("baselineState".to_string(), Value::String(state.as_str().into()));
    }

    value
}

fn physical_location(diagnostic: &Diagnostic) -> Value {
    let mut physical = json!({
        "artifactLocation": { "uri": path_to_file_uri(&diagnostic.path) },
    });
    if let Some(position) = &diagnostic.position {
        physical.as_object_mut().expect("object").insert(
            "region".to_string(),
            json!({
                "startLine": position.line,
                "startColumn": position.column,
            }),
        );
    }
    physical
}

fn path_to_file_uri(path: &Path) -> String {
    let normalized = path
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .to_string();
    if normalized.starts_with("file://") {
        normalized
    } else if normalized.starts_with('/') {
        format!("file://{normalized}")
    } else {
        format!("file:///{normalized}")
    }
}

fn sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::{Diagnostic, Position, Severity};
    use std::path::PathBuf;

    fn make_metadata() -> RunMetadata {
        RunMetadata::capture("native", false, false, "sarif", None, None)
    }

    #[test]
    fn emits_minimal_valid_document() {
        let metadata = make_metadata();
        let output = emit(
            &[],
            &metadata,
            "sysml-validate validate .",
            0,
            "2026-05-18T13:00:00Z",
            None,
            None,
        );
        let parsed: Value = serde_json::from_str(&output).expect("parse SARIF");
        assert_eq!(parsed["version"], "2.1.0");
        assert_eq!(parsed["runs"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["runs"][0]["tool"]["driver"]["name"], "sysml-validate");
        let rules = parsed["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules array");
        assert!(!rules.is_empty());
        assert!(rules.iter().any(|rule| rule["id"] == "SYSML041"));
    }

    #[test]
    fn emits_diagnostic_with_fingerprint() {
        let path = PathBuf::from("/project/model.sysml");
        let mut result = ValidationResult::new(&path);
        result.diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name 'Engine' in the same lexical scope.",
            &path,
            Some(Position { line: 5, column: 1 }),
        ));
        let metadata = make_metadata();
        let output = emit(
            &[result],
            &metadata,
            "sysml-validate validate ./project",
            1,
            "2026-05-18T13:00:01Z",
            None,
            None,
        );
        let parsed: Value = serde_json::from_str(&output).expect("parse SARIF");
        let result_entry = &parsed["runs"][0]["results"][0];
        assert_eq!(result_entry["ruleId"], "SYSML041");
        assert_eq!(result_entry["level"], "error");
        assert!(result_entry["partialFingerprints"]["diagnosticHash/v1"].is_string());
        assert_eq!(result_entry["locations"][0]["physicalLocation"]["region"]["startLine"], 5);
        // No baseline given → no baselineState field.
        assert!(result_entry.get("baselineState").is_none());
        assert!(result_entry.get("suppressions").is_none());
    }

    #[test]
    fn emits_suppression_marker_when_diagnostic_is_suppressed() {
        let path = PathBuf::from("/project/model.sysml");
        let mut diagnostic = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name 'Engine'.",
            &path,
            Some(Position { line: 5, column: 1 }),
        );
        diagnostic.suppression = Some("Suppressed by directive at line 4 column 1.".into());
        let mut result = ValidationResult::new(&path);
        result.diagnostics.push(diagnostic);

        let metadata = make_metadata();
        let output = emit(
            &[result],
            &metadata,
            "sysml-validate validate .",
            0,
            "2026-05-18T13:00:00Z",
            None,
            None,
        );
        let parsed: Value = serde_json::from_str(&output).unwrap();
        let suppressions = &parsed["runs"][0]["results"][0]["suppressions"];
        assert_eq!(suppressions[0]["kind"], "inSource");
        assert_eq!(suppressions[0]["status"], "accepted");
    }

    #[test]
    fn baseline_classifies_new_vs_unchanged() {
        use crate::baseline::Baseline;

        let path = PathBuf::from("/project/model.sysml");
        let mut result = ValidationResult::new(&path);
        result.diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name 'Engine'.",
            &path,
            Some(Position { line: 5, column: 1 }),
        ));

        // First emit a SARIF doc with no baseline so we can extract the
        // diagnostic's fingerprint and build a baseline from it.
        let metadata = make_metadata();
        let first = emit(
            &[result.clone()],
            &metadata,
            "sysml-validate validate .",
            1,
            "2026-05-18T13:00:00Z",
            None,
            None,
        );
        let parsed_first: Value = serde_json::from_str(&first).unwrap();
        let fingerprint = parsed_first["runs"][0]["results"][0]["partialFingerprints"]
            ["diagnosticHash/v1"]
            .as_str()
            .unwrap()
            .to_string();

        let baseline_doc = serde_json::json!({
            "version": "2.1.0",
            "runs": [
                {
                    "results": [
                        {
                            "ruleId": "SYSML041",
                            "partialFingerprints": { "diagnosticHash/v1": fingerprint }
                        }
                    ]
                }
            ]
        });
        let baseline = Baseline::from_sarif(&baseline_doc).unwrap();

        let with_baseline = emit(
            &[result],
            &metadata,
            "sysml-validate validate .",
            0,
            "2026-05-18T13:00:00Z",
            None,
            Some(&baseline),
        );
        let parsed: Value = serde_json::from_str(&with_baseline).unwrap();
        assert_eq!(parsed["runs"][0]["results"][0]["baselineState"], "unchanged");
    }

    #[test]
    fn maps_severity_to_sarif_level() {
        assert_eq!(sarif_level(Severity::Error), "error");
        assert_eq!(sarif_level(Severity::Warning), "warning");
        assert_eq!(sarif_level(Severity::Info), "note");
    }

    #[test]
    fn windows_path_becomes_file_uri() {
        let uri = path_to_file_uri(Path::new("C:\\Users\\mike\\model.sysml"));
        assert_eq!(uri, "file:///C:/Users/mike/model.sysml");
    }
}
