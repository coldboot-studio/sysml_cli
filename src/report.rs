use std::fmt::Write as _;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::baseline::Baseline;
use crate::diag::ValidationResult;
use crate::junit;
use crate::sarif;

/// Rule catalog version. Bump when the meaning of any SYSML* rule code
/// changes, when codes are added, or when codes are removed. Consumers can
/// gate their baselines on this value.
///
/// 0.2.0: added SYSML904 (official-backend timeout).
/// 0.3.0: added SYSML050 (unused suppression), SYSML060 (invalid
///        suppression directive), SYSML800 (invalid config file).
/// 0.4.0: added SYSML210/211 (target-missing for specialization /
///        redefinition), SYSML212/213 (self-reference),
///        SYSML220 (specialization cycle across project).
pub const RULE_CATALOG_VERSION: &str = "0.4.0";

pub struct RunMetadata {
    pub tool_name: &'static str,
    pub tool_version: &'static str,
    pub rule_catalog_version: &'static str,
    pub timestamp_utc: String,
    pub timestamp_epoch_seconds: u64,
    pub backend: &'static str,
    pub strict: bool,
    pub fail_on_warning: bool,
    pub format: &'static str,
    pub config_path: Option<String>,
    pub baseline_path: Option<String>,
}

impl RunMetadata {
    pub fn capture(
        backend: &'static str,
        strict: bool,
        fail_on_warning: bool,
        format: &'static str,
        config_path: Option<String>,
        baseline_path: Option<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let epoch = now.as_secs();
        Self {
            tool_name: env!("CARGO_PKG_NAME"),
            tool_version: env!("CARGO_PKG_VERSION"),
            rule_catalog_version: RULE_CATALOG_VERSION,
            timestamp_utc: format_rfc3339_utc(epoch),
            timestamp_epoch_seconds: epoch,
            backend,
            strict,
            fail_on_warning,
            format,
            config_path,
            baseline_path,
        }
    }
}

pub fn print_sarif_results(
    results: &[ValidationResult],
    metadata: &RunMetadata,
    command_line: &str,
    project_root: Option<&Path>,
    baseline: Option<&Baseline>,
    exit_code: i32,
) -> String {
    let end = format_rfc3339_utc(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    );
    let document = sarif::emit(
        results,
        metadata,
        command_line,
        exit_code,
        &end,
        project_root,
        baseline,
    );
    println!("{document}");
    document
}

pub fn print_junit_results(results: &[ValidationResult], fail_on_warning: bool) {
    let document = junit::emit(results, fail_on_warning);
    print!("{document}");
}

/// Screen-reader-friendly output (US-114). No metadata header, no decoration,
/// no color signaling. One diagnostic per line in a deterministic GCC-style
/// format that screen readers and IDEs already understand:
///
/// ```text
/// <path>:<line>:<column>: <severity>: <code>: <message>
/// ```
///
/// Diagnostics with no position render as `<path>: ...`. Suppressed
/// diagnostics are excluded unless `--show-suppressed` is set, in which case
/// they appear with a ` [suppressed]` suffix on the severity field.
pub fn print_plain_results(results: &[ValidationResult], show_suppressed: bool) {
    for result in results {
        for diagnostic in &result.diagnostics {
            if diagnostic.is_suppressed() && !show_suppressed {
                continue;
            }
            let location = diagnostic
                .position
                .as_ref()
                .map(|position| format!(":{}:{}", position.line, position.column))
                .unwrap_or_default();
            let suppression_marker = if diagnostic.is_suppressed() {
                " [suppressed]"
            } else {
                ""
            };
            println!(
                "{}{}: {}{}: {}: {}",
                diagnostic.path.display(),
                location,
                diagnostic.severity.as_str(),
                suppression_marker,
                diagnostic.code,
                diagnostic.message,
            );
        }
    }
}

pub fn print_text_results(
    results: &[ValidationResult],
    metadata: &RunMetadata,
    show_suppressed: bool,
) {
    println!(
        "{} {} (rules {}) backend={} strict={} fail-on-warning={} at {}",
        metadata.tool_name,
        metadata.tool_version,
        metadata.rule_catalog_version,
        metadata.backend,
        metadata.strict,
        metadata.fail_on_warning,
        metadata.timestamp_utc,
    );
    if let Some(path) = &metadata.config_path {
        println!("  config: {path}");
    }
    if let Some(path) = &metadata.baseline_path {
        println!("  baseline: {path}");
    }
    for result in results {
        let status = if result.ok() { "OK" } else { "FAIL" };
        println!("{status} {}", result.path.display());
        for diagnostic in &result.diagnostics {
            if diagnostic.is_suppressed() && !show_suppressed {
                continue;
            }
            let location = diagnostic
                .position
                .as_ref()
                .map(|position| format!(":{}:{}", position.line, position.column))
                .unwrap_or_default();
            let suppression_marker = if diagnostic.is_suppressed() {
                " [suppressed]"
            } else {
                ""
            };
            println!(
                "  {} {}{}{} {}",
                diagnostic.severity.as_str().to_ascii_uppercase(),
                diagnostic.code,
                location,
                suppression_marker,
                diagnostic.message
            );
        }
    }
    let errors: usize = results.iter().map(ValidationResult::error_count).sum();
    let warnings: usize = results.iter().map(ValidationResult::warning_count).sum();
    let suppressed: usize = results.iter().map(ValidationResult::suppressed_count).sum();
    println!(
        "\nValidated {} file(s): {} error(s), {} warning(s), {} suppressed.",
        results.len(),
        errors,
        warnings,
        suppressed,
    );
}

pub fn print_json_results(
    results: &[ValidationResult],
    metadata: &RunMetadata,
    show_suppressed: bool,
) {
    let mut output = String::new();
    output.push_str("{\n");
    let config_path_field = match &metadata.config_path {
        Some(path) => format!(", \"config_path\": \"{}\"", json_escape(path)),
        None => String::new(),
    };
    let baseline_path_field = match &metadata.baseline_path {
        Some(path) => format!(", \"baseline_path\": \"{}\"", json_escape(path)),
        None => String::new(),
    };
    write!(
        output,
        "  \"metadata\": {{\n    \"tool\": {{\"name\": \"{}\", \"version\": \"{}\"}},\n    \"rule_catalog\": {{\"version\": \"{}\"}},\n    \"invocation\": {{\"timestamp_utc\": \"{}\", \"timestamp_epoch_seconds\": {}, \"backend\": \"{}\", \"strict\": {}, \"fail_on_warning\": {}, \"format\": \"{}\"{}{}}}\n  }},\n",
        json_escape(metadata.tool_name),
        json_escape(metadata.tool_version),
        json_escape(metadata.rule_catalog_version),
        json_escape(&metadata.timestamp_utc),
        metadata.timestamp_epoch_seconds,
        json_escape(metadata.backend),
        metadata.strict,
        metadata.fail_on_warning,
        json_escape(metadata.format),
        config_path_field,
        baseline_path_field,
    )
    .expect("write to String cannot fail");
    output.push_str("  \"results\": [\n");
    for (index, result) in results.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        let visible: Vec<&_> = result
            .diagnostics
            .iter()
            .filter(|diagnostic| show_suppressed || !diagnostic.is_suppressed())
            .collect();
        write!(
            output,
            "    {{\n      \"path\": \"{}\",\n      \"ok\": {},\n      \"error_count\": {},\n      \"warning_count\": {},\n      \"suppressed_count\": {},\n      \"diagnostics\": [",
            json_escape(&result.path.to_string_lossy()),
            result.ok(),
            result.error_count(),
            result.warning_count(),
            result.suppressed_count(),
        )
        .expect("write to String cannot fail");
        if !visible.is_empty() {
            output.push('\n');
        }
        for (diagnostic_index, diagnostic) in visible.iter().enumerate() {
            if diagnostic_index > 0 {
                output.push_str(",\n");
            }
            write!(
                output,
                "        {{\"severity\": \"{}\", \"code\": \"{}\", \"fingerprint\": \"{}\", \"message\": \"{}\", \"path\": \"{}\"",
                diagnostic.severity.as_str(),
                diagnostic.code,
                diagnostic.fingerprint(None),
                json_escape(&diagnostic.message),
                json_escape(&diagnostic.path.to_string_lossy())
            )
            .expect("write to String cannot fail");
            if let Some(position) = &diagnostic.position {
                write!(
                    output,
                    ", \"position\": {{\"line\": {}, \"column\": {}}}",
                    position.line, position.column
                )
                .expect("write to String cannot fail");
            }
            if let Some(reason) = &diagnostic.suppression {
                write!(
                    output,
                    ", \"suppression\": {{\"justification\": \"{}\"}}",
                    json_escape(reason)
                )
                .expect("write to String cannot fail");
            }
            output.push('}');
        }
        if !visible.is_empty() {
            output.push('\n');
            output.push_str("      ");
        }
        output.push_str("]\n    }");
    }
    output.push_str("\n  ]\n}");
    println!("{output}");
}

pub fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => {
                write!(escaped, "\\u{:04x}", c as u32).expect("write to String cannot fail");
            }
            c => escaped.push(c),
        }
    }
    escaped
}

/// Format a Unix epoch second count as an RFC 3339 / ISO 8601 UTC string.
///
/// Uses Howard Hinnant's civil_from_days algorithm. Valid for the proleptic
/// Gregorian calendar, no leap seconds.
fn format_rfc3339_utc(epoch_seconds: u64) -> String {
    let secs_per_day: u64 = 86_400;
    let days = (epoch_seconds / secs_per_day) as i64;
    let time_of_day = epoch_seconds % secs_per_day;
    let hour = (time_of_day / 3600) as u32;
    let minute = ((time_of_day % 3600) / 60) as u32;
    let second = (time_of_day % 60) as u32;

    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_unix_epoch() {
        assert_eq!(format_rfc3339_utc(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn formats_known_timestamp() {
        // 2026-05-18T00:00:00Z
        assert_eq!(format_rfc3339_utc(1_779_062_400), "2026-05-18T00:00:00Z");
    }

    #[test]
    fn formats_leap_day_boundary() {
        // 2024-02-29T12:34:56Z
        assert_eq!(format_rfc3339_utc(1_709_210_096), "2024-02-29T12:34:56Z");
    }
}
