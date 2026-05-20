use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use wait_timeout::ChildExt;

use crate::diag::{Diagnostic, Position, Severity, ValidationResult};

pub const DEFAULT_OFFICIAL_TIMEOUT_SECONDS: u64 = 60;

pub fn validate_official(
    path: &Path,
    command_template: &str,
    timeout: Duration,
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
        .spawn();

    let mut child = match spawn {
        Ok(child) => child,
        Err(error) => {
            result.diagnostics.push(Diagnostic::new(
                Severity::Error,
                "SYSML901",
                format!("Unable to execute official validator: {error}"),
                path,
                None,
            ));
            return result;
        }
    };

    // Drain stdout/stderr in worker threads so a chatty child cannot block
    // on a full pipe while we wait for exit.
    let stdout_handle = child.stdout.take().map(|mut stream| {
        thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = stream.read_to_end(&mut buffer);
            buffer
        })
    });
    let stderr_handle = child.stderr.take().map(|mut stream| {
        thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = stream.read_to_end(&mut buffer);
            buffer
        })
    });

    let status = match child.wait_timeout(timeout) {
        Ok(Some(status)) => status,
        Ok(None) => {
            let _ = child.kill();
            // Drain the readers so the threads exit cleanly. Their output is
            // discarded; the timeout is the dominant signal.
            if let Some(handle) = stdout_handle {
                let _ = handle.join();
            }
            if let Some(handle) = stderr_handle {
                let _ = handle.join();
            }
            let _ = child.wait();
            result.diagnostics.push(Diagnostic::new(
                Severity::Error,
                "SYSML904",
                format!(
                    "Official validator exceeded --timeout ({} s) and was terminated.",
                    timeout.as_secs()
                ),
                path,
                Some(Position { line: 1, column: 1 }),
            ));
            return result;
        }
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            result.diagnostics.push(Diagnostic::new(
                Severity::Error,
                "SYSML901",
                format!("Unable to wait for official validator: {error}"),
                path,
                None,
            ));
            return result;
        }
    };

    let stdout = stdout_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    let stderr = stderr_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    let message = command_output_message(&stdout, &stderr);

    if status.success() {
        if !message.is_empty() {
            result.diagnostics.push(Diagnostic::new(
                Severity::Info,
                "SYSML903",
                message,
                path,
                Some(Position { line: 1, column: 1 }),
            ));
        }
    } else {
        result.diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML902",
            if message.is_empty() {
                format!("Official validator exited with status {}.", status)
            } else {
                message
            },
            path,
            Some(Position { line: 1, column: 1 }),
        ));
    }

    result
}

fn command_output_message(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
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
/// quotes (only for `\`, `"`, `$`, backtick — matching POSIX `sh`). Does
/// NOT spawn a shell, so command-template inputs cannot inject shell
/// metacharacters into a child shell process.
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
            '\\' if in_double => match chars.peek() {
                Some(&next) if matches!(next, '\\' | '"' | '$' | '`') => {
                    current.push(next);
                    chars.next();
                }
                _ => current.push('\\'),
            },
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
        // Metacharacters survive as literal argv content. They are not
        // interpreted because we never invoke a shell.
        let argv = shlex_split("validator 'x; rm -rf /'").unwrap();
        assert_eq!(argv, vec!["validator", "x; rm -rf /"]);
    }

    #[test]
    fn timeout_kills_slow_child() {
        let path = std::path::Path::new("test://slow");
        // PowerShell is available on Windows hosts; Start-Sleep blocks for
        // 30 s, well beyond our 1 s timeout.
        #[cfg(windows)]
        let template = "powershell -NoProfile -Command \"Start-Sleep 30\"";
        #[cfg(not(windows))]
        let template = "sleep 30";

        let start = std::time::Instant::now();
        let result = validate_official(path, template, Duration::from_secs(1));
        let elapsed = start.elapsed();

        // The timeout must fire well within the 30 s sleep. Allow generous
        // headroom for slow CI.
        assert!(
            elapsed < Duration::from_secs(15),
            "validate_official did not honor --timeout: elapsed={elapsed:?}"
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "SYSML904"),
            "expected SYSML904 timeout diagnostic, got: {:?}",
            result.diagnostics
        );
    }
}
