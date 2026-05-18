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
