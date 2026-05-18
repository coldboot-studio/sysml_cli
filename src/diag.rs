use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub path: PathBuf,
    pub position: Option<Position>,
}

impl Diagnostic {
    pub fn new(
        severity: Severity,
        code: &'static str,
        message: impl Into<String>,
        path: &Path,
        position: Option<Position>,
    ) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            path: path.to_path_buf(),
            position,
        }
    }

    /// Compute a stable per-diagnostic fingerprint for SARIF
    /// `partialFingerprints.diagnosticHash/v1` and for baseline diff matching.
    ///
    /// **Stability v1**: hash of (rule code, file path normalized to project
    /// root if known, message template with literal identifiers and numbers
    /// genericized). The fingerprint does **not** depend on the diagnostic's
    /// position, so inserting an unrelated line above will not change the
    /// fingerprint of a diagnostic that follows. A given file may have
    /// multiple diagnostics with identical fingerprints if the same rule
    /// fires for the same genericized message; consumers should disambiguate
    /// using line numbers when they care.
    pub fn fingerprint(&self, project_root: Option<&Path>) -> String {
        let normalized_path = normalize_path(&self.path, project_root);
        let genericized_message = genericize_message(&self.message);

        let mut hasher = Sha256::new();
        hasher.update(b"sysml-validate/diagnostic/v1\n");
        hasher.update(self.code.as_bytes());
        hasher.update(b"\n");
        hasher.update(normalized_path.as_bytes());
        hasher.update(b"\n");
        hasher.update(genericized_message.as_bytes());

        let digest = hasher.finalize();
        // 16 hex chars (64 bits) is enough collision resistance for a
        // per-project diagnostic identifier and is short enough to read.
        digest[..8]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

fn normalize_path(path: &Path, project_root: Option<&Path>) -> String {
    let trimmed = match project_root {
        Some(root) => path.strip_prefix(root).unwrap_or(path),
        None => path,
    };
    // Force forward-slash separators so the fingerprint is identical on
    // Windows and POSIX hosts.
    trimmed
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .to_string()
}

/// Replace literal identifiers and numbers in a diagnostic message with
/// placeholders so the fingerprint is stable across input renamings.
///
/// Conservative v1: replace anything between single or double quotes with
/// `<lit>`, and any integer or decimal run with `<num>`. Future versions
/// may operate on a structured message template.
fn genericize_message(message: &str) -> String {
    let mut output = String::with_capacity(message.len());
    let mut chars = message.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\'' | '"' => {
                output.push_str("<lit>");
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == character {
                        break;
                    }
                }
            }
            c if c.is_ascii_digit() => {
                output.push_str("<num>");
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() || next == '.' {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            c => output.push(c),
        }
    }
    output
}

#[derive(Clone, Debug)]
pub struct ValidationResult {
    pub path: PathBuf,
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationResult {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            diagnostics: Vec::new(),
        }
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Warning)
            .count()
    }

    pub fn ok(&self) -> bool {
        self.error_count() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_deterministic() {
        let path = PathBuf::from("/project/model.sysml");
        let diagnostic = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name 'Engine' in the same lexical scope.",
            &path,
            Some(Position { line: 5, column: 1 }),
        );
        let first = diagnostic.fingerprint(None);
        let second = diagnostic.fingerprint(None);
        assert_eq!(first, second);
    }

    #[test]
    fn fingerprint_ignores_position() {
        let path = PathBuf::from("/project/model.sysml");
        let a = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name 'Engine' in the same lexical scope.",
            &path,
            Some(Position { line: 5, column: 1 }),
        );
        let b = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name 'Engine' in the same lexical scope.",
            &path,
            Some(Position {
                line: 99,
                column: 42,
            }),
        );
        assert_eq!(a.fingerprint(None), b.fingerprint(None));
    }

    #[test]
    fn fingerprint_ignores_literal_identifiers() {
        let path = PathBuf::from("/project/model.sysml");
        let engine = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name 'Engine' in the same lexical scope.",
            &path,
            None,
        );
        let wheel = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name 'Wheel' in the same lexical scope.",
            &path,
            None,
        );
        assert_eq!(engine.fingerprint(None), wheel.fingerprint(None));
    }

    #[test]
    fn fingerprint_differs_by_rule_code() {
        let path = PathBuf::from("/project/model.sysml");
        let a = Diagnostic::new(Severity::Error, "SYSML041", "x", &path, None);
        let b = Diagnostic::new(Severity::Error, "SYSML030", "x", &path, None);
        assert_ne!(a.fingerprint(None), b.fingerprint(None));
    }

    #[test]
    fn fingerprint_normalizes_path_separators() {
        let win = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "x",
            Path::new("C:\\project\\model.sysml"),
            None,
        );
        let posix = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "x",
            Path::new("C:/project/model.sysml"),
            None,
        );
        assert_eq!(win.fingerprint(None), posix.fingerprint(None));
    }

    #[test]
    fn fingerprint_respects_project_root() {
        let absolute = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "x",
            Path::new("/project/sub/model.sysml"),
            None,
        );
        let with_root = absolute.fingerprint(Some(Path::new("/project")));
        let without_root = absolute.fingerprint(None);
        assert_ne!(with_root, without_root);
    }
}
