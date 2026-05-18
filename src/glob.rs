//! Minimal glob matcher for config-driven `include` / `exclude` patterns.
//!
//! Supported syntax:
//! - `*`   — matches zero or more characters within a single path segment
//! - `?`   — matches exactly one character within a single path segment
//! - `**`  — matches zero or more path segments (including separators)
//! - `[...]` is NOT supported in v1
//!
//! Path separators are normalized to `/` before matching so a single
//! glob string works on Windows and POSIX hosts. Matching is performed
//! against the full path (typically relative to the project root).

use std::path::Path;

#[derive(Clone, Debug)]
pub struct Pattern {
    #[allow(dead_code)] // exposed via as_str() for diagnostic messages.
    raw: String,
    parts: Vec<Part>,
}

#[derive(Clone, Debug)]
enum Part {
    Literal(String),
    AnySegmentChars,
    AnyChar,
    DoubleStar,
    Separator,
}

impl Pattern {
    pub fn new(raw: &str) -> Self {
        let mut parts = Vec::new();
        let mut buffer = String::new();
        let mut chars = raw.chars().peekable();
        while let Some(character) = chars.next() {
            match character {
                '/' | '\\' => {
                    flush_literal(&mut buffer, &mut parts);
                    parts.push(Part::Separator);
                }
                '*' => {
                    flush_literal(&mut buffer, &mut parts);
                    if chars.peek() == Some(&'*') {
                        chars.next();
                        parts.push(Part::DoubleStar);
                    } else {
                        parts.push(Part::AnySegmentChars);
                    }
                }
                '?' => {
                    flush_literal(&mut buffer, &mut parts);
                    parts.push(Part::AnyChar);
                }
                character => buffer.push(character),
            }
        }
        flush_literal(&mut buffer, &mut parts);
        Self {
            raw: raw.to_string(),
            parts,
        }
    }

    pub fn matches(&self, path: &str) -> bool {
        let normalized = path.replace('\\', "/");
        let normalized = normalized.trim_start_matches("//?/");
        match_parts(&self.parts, normalized.as_bytes())
    }

    #[allow(dead_code)] // public API; Phase 2 file walker will consume.
    pub fn matches_path(&self, path: &Path) -> bool {
        self.matches(&path.to_string_lossy())
    }

    #[allow(dead_code)] // public API; useful for error messages downstream.
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

fn flush_literal(buffer: &mut String, parts: &mut Vec<Part>) {
    if !buffer.is_empty() {
        parts.push(Part::Literal(std::mem::take(buffer)));
    }
}

fn match_parts(parts: &[Part], input: &[u8]) -> bool {
    match parts.first() {
        None => input.is_empty(),
        Some(Part::Literal(literal)) => {
            let bytes = literal.as_bytes();
            input.starts_with(bytes) && match_parts(&parts[1..], &input[bytes.len()..])
        }
        Some(Part::Separator) => {
            input.first() == Some(&b'/') && match_parts(&parts[1..], &input[1..])
        }
        Some(Part::AnyChar) => {
            !input.is_empty() && input[0] != b'/' && match_parts(&parts[1..], &input[1..])
        }
        Some(Part::AnySegmentChars) => {
            // Match zero or more non-separator characters.
            for cut in 0..=input.len() {
                if cut > 0 && input[cut - 1] == b'/' {
                    break;
                }
                if match_parts(&parts[1..], &input[cut..]) {
                    return true;
                }
            }
            false
        }
        Some(Part::DoubleStar) => {
            // `**` matches zero or more characters including separators.
            // Skip optional trailing separator after the `**` so
            // `src/**/foo` works as well as `src/**foo`.
            let rest = if matches!(parts.get(1), Some(Part::Separator)) {
                &parts[2..]
            } else {
                &parts[1..]
            };
            for cut in 0..=input.len() {
                if match_parts(rest, &input[cut..]) {
                    return true;
                }
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_matches_exact() {
        assert!(Pattern::new("src/model.sysml").matches("src/model.sysml"));
        assert!(!Pattern::new("src/model.sysml").matches("src/other.sysml"));
    }

    #[test]
    fn star_matches_within_segment() {
        let pattern = Pattern::new("src/*.sysml");
        assert!(pattern.matches("src/model.sysml"));
        assert!(pattern.matches("src/other.sysml"));
        assert!(!pattern.matches("src/nested/model.sysml"));
        assert!(!pattern.matches("model.sysml"));
    }

    #[test]
    fn double_star_crosses_segments() {
        let pattern = Pattern::new("**/*.sysml");
        assert!(pattern.matches("model.sysml"));
        assert!(pattern.matches("a/b/c/model.sysml"));
        assert!(!pattern.matches("model.kerml"));
    }

    #[test]
    fn double_star_in_middle() {
        let pattern = Pattern::new("src/**/model.sysml");
        assert!(pattern.matches("src/model.sysml"));
        assert!(pattern.matches("src/a/model.sysml"));
        assert!(pattern.matches("src/a/b/c/model.sysml"));
        assert!(!pattern.matches("model.sysml"));
    }

    #[test]
    fn question_mark_single_char() {
        let pattern = Pattern::new("?ar.sysml");
        assert!(pattern.matches("car.sysml"));
        assert!(pattern.matches("bar.sysml"));
        assert!(!pattern.matches("scar.sysml"));
        assert!(!pattern.matches("ar.sysml"));
    }

    #[test]
    fn normalizes_windows_separators() {
        let pattern = Pattern::new("src/**/*.sysml");
        assert!(pattern.matches("src\\a\\b.sysml"));
    }

    #[test]
    fn strips_windows_unc_prefix() {
        let pattern = Pattern::new("**/*.sysml");
        assert!(pattern.matches("//?/C:/Users/mike/model.sysml"));
    }

    #[test]
    fn exclude_target_directory() {
        let pattern = Pattern::new("**/target/**");
        assert!(pattern.matches("target/debug/foo.sysml"));
        assert!(pattern.matches("crate/target/debug/foo.sysml"));
        assert!(!pattern.matches("src/foo.sysml"));
    }
}
