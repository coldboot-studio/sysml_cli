//! SARIF baseline / diff mode (US-106).
//!
//! A baseline is a previous SARIF log. Loading a baseline collects the
//! `(ruleId, partialFingerprints.diagnosticHash/v1)` pairs from every
//! result, which the current run uses to classify findings as `new`,
//! `unchanged`, or `absent`. Only `new` findings affect the exit code.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BaselineState {
    /// The diagnostic does not appear in the baseline.
    New,
    /// The baseline contained a matching diagnostic.
    Unchanged,
}

impl BaselineState {
    pub fn as_str(self) -> &'static str {
        match self {
            BaselineState::New => "new",
            BaselineState::Unchanged => "unchanged",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Baseline {
    entries: HashSet<BaselineKey>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct BaselineKey {
    rule_id: String,
    fingerprint: String,
}

impl Baseline {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("unable to read baseline '{}': {error}", path.display()))?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|error| format!("baseline '{}' is not valid JSON: {error}", path.display()))?;
        Self::from_sarif(&value)
            .map_err(|error| format!("baseline '{}' is not valid SARIF: {error}", path.display()))
    }

    pub fn from_sarif(value: &Value) -> Result<Self, String> {
        let runs = value
            .get("runs")
            .and_then(Value::as_array)
            .ok_or_else(|| "missing 'runs' array".to_string())?;
        let mut entries = HashSet::new();
        for run in runs {
            let Some(results) = run.get("results").and_then(Value::as_array) else {
                continue;
            };
            for result in results {
                let Some(rule_id) = result.get("ruleId").and_then(Value::as_str) else {
                    continue;
                };
                let Some(fingerprint) = result
                    .get("partialFingerprints")
                    .and_then(|fingerprints| fingerprints.get("diagnosticHash/v1"))
                    .and_then(Value::as_str)
                else {
                    continue;
                };
                entries.insert(BaselineKey {
                    rule_id: rule_id.to_string(),
                    fingerprint: fingerprint.to_string(),
                });
            }
        }
        Ok(Self { entries })
    }

    pub fn classify(&self, rule_id: &str, fingerprint: &str) -> BaselineState {
        let key = BaselineKey {
            rule_id: rule_id.to_string(),
            fingerprint: fingerprint.to_string(),
        };
        if self.entries.contains(&key) {
            BaselineState::Unchanged
        } else {
            BaselineState::New
        }
    }

    #[allow(dead_code)] // public API used in tests; may be consumed in Phase 2 LSP.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        json!({
            "version": "2.1.0",
            "runs": [
                {
                    "results": [
                        {
                            "ruleId": "SYSML041",
                            "partialFingerprints": { "diagnosticHash/v1": "abc123" }
                        },
                        {
                            "ruleId": "SYSML040",
                            "partialFingerprints": { "diagnosticHash/v1": "def456" }
                        }
                    ]
                }
            ]
        })
    }

    #[test]
    fn parses_sarif_fingerprints() {
        let baseline = Baseline::from_sarif(&fixture()).unwrap();
        assert!(!baseline.is_empty());
        assert_eq!(
            baseline.classify("SYSML041", "abc123"),
            BaselineState::Unchanged
        );
        assert_eq!(
            baseline.classify("SYSML041", "different"),
            BaselineState::New
        );
        assert_eq!(baseline.classify("SYSML030", "abc123"), BaselineState::New);
    }

    #[test]
    fn missing_runs_array_is_error() {
        let value = json!({"version": "2.1.0"});
        assert!(Baseline::from_sarif(&value).is_err());
    }

    #[test]
    fn empty_baseline_classifies_everything_new() {
        let baseline = Baseline::empty();
        assert!(baseline.is_empty());
        assert_eq!(baseline.classify("X", "Y"), BaselineState::New);
    }
}
