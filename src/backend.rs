use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::diag::{Diagnostic, Position, Severity, ValidationResult};

pub const DEFAULT_OFFICIAL_TIMEOUT_SECONDS: u64 = 60;

pub fn validate_official(
    path: &Path,
    command_template: &str,
    _timeout: Duration,
) -> ValidationResult {
    let mut result = ValidationResult::new(path);

    let argv = match shlex_split(command_template) {
        Ok(argv) => argv,
        Err(message) => {
            result.diagnostics.push(Diagnostic::new(
                Severity::Error,
                "SYSML900",
                message,
                path,
                None,
            ));
            return result;
        }
    };
    if argv.is_empty() {
        result.diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML900",
            "--official-command must include an executable.",
            path,
            None,
        ));
        return result;
    }

    let file = path.to_string_lossy().to_string();
    let substituted: Vec<String> = argv
        .into_iter()
        .map(|argument| argument.replace("{file}", &file))
        .collect();
    let (program, args) = substituted.split_first().expect("argv is non-empty");

    let spawn = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match spawn {
        Ok(output) if output.status.success() => {
            let message = command_output_message(&output);
            if !message.is_empty() {
                result.diagnostics.push(Diagnostic::new(
                    Severity::Info,
                    "SYSML903",
                    message,
                    path,
                    Some(Position { line: 1, column: 1 }),
                ));
            }
        }
        Ok(output) => {
            let message = command_output_message(&output);
            result.diagnostics.push(Diagnostic::new(
                Severity::Error,
                "SYSML902",
                if message.is_empty() {
                    format!("Official validator exited with status {}.", output.status)
                } else {
                    message
                },
                path,
                Some(Position { line: 1, column: 1 }),
            ));
        }
        Err(error) => {
            result.diagnostics.push(Diagnostic::new(
                Severity::Error,
                "SYSML901",
                format!("Unable to execute official validator: {error}"),
                path,
                None,
            ));
        }
    }

    result
}

fn command_output_message(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout,
        (true, false) => stderr,
        (false, false) => format!("{stdout}\n{stderr}"),
    }
}

/// Minimal shell-style tokenizer for `--official-command` templates.
///
/// Honors single quotes, double quotes, and backslash escapes inside double
/// quotes. Does NOT spawn a shell, so command-template inputs cannot inject
/// shell metacharacters into a child shell process.
pub fn shlex_split(input: &str) -> Result<Vec<String>, String> {
    let mut argv = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut has_token = false;

    while let Some(character) = chars.next() {
        match character {
            '\'' if !in_double => {
                in_single = !in_single;
                has_token = true;
            }
            '"' if !in_single => {
                in_double = !in_double;
                has_token = true;
            }
            '\\' if in_double => {
                match chars.peek() {
                    Some(&next) if matches!(next, '\\' | '"' | '$' | '`') => {
                        current.push(next);
                        chars.next();
                    }
                    _ => current.push('\\'),
                }
            }
            character if character.is_whitespace() && !in_single && !in_double => {
                if has_token {
                    argv.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            character => {
                current.push(character);
                has_token = true;
            }
        }
    }
    if in_single || in_double {
        return Err("--official-command has an unterminated quoted argument.".to_string());
    }
    if has_token {
        argv.push(current);
    }
    Ok(argv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shlex_simple() {
        let argv = shlex_split("validator --flag value").unwrap();
        assert_eq!(argv, vec!["validator", "--flag", "value"]);
    }

    #[test]
    fn shlex_double_quotes() {
        let argv = shlex_split(r#"validator --path "C:\Program Files\tool" --flag"#).unwrap();
        assert_eq!(
            argv,
            vec!["validator", "--path", "C:\\Program Files\\tool", "--flag"]
        );
    }

    #[test]
    fn shlex_single_quotes() {
        let argv = shlex_split("validator '--flag with spaces' tail").unwrap();
        assert_eq!(argv, vec!["validator", "--flag with spaces", "tail"]);
    }

    #[test]
    fn shlex_rejects_unterminated_quote() {
        assert!(shlex_split("validator \"unterminated").is_err());
    }

    #[test]
    fn shlex_preserves_empty_quoted_arg() {
        let argv = shlex_split("validator ''").unwrap();
        assert_eq!(argv, vec!["validator", ""]);
    }

    #[test]
    fn shlex_rejects_shell_metacharacter_injection() {
        // The metacharacters survive as literal argv content. They are NOT
        // interpreted because we never invoke a shell.
        let argv = shlex_split("validator 'x; rm -rf /'").unwrap();
        assert_eq!(argv, vec!["validator", "x; rm -rf /"]);
    }
}
