use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::diag::ValidationResult;

/// Rule catalog version. Bump when the meaning of any SYSML* rule code
/// changes, when codes are added, or when codes are removed. Consumers can
/// gate their baselines on this value.
pub const RULE_CATALOG_VERSION: &str = "0.1.0";

pub struct RunMetadata {
    pub tool_name: &'static str,
    pub tool_version: &'static str,
    pub rule_catalog_version: &'static str,
    pub timestamp_utc: String,
    pub timestamp_epoch_seconds: u64,
    pub backend: &'static str,
    pub strict: bool,
    pub format: &'static str,
}

impl RunMetadata {
    pub fn capture(backend: &'static str, strict: bool, format: &'static str) -> Self {
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
            format,
        }
    }
}

pub fn print_text_results(results: &[ValidationResult], metadata: &RunMetadata) {
    println!(
        "{} {} (rules {}) backend={} strict={} at {}",
        metadata.tool_name,
        metadata.tool_version,
        metadata.rule_catalog_version,
        metadata.backend,
        metadata.strict,
        metadata.timestamp_utc,
    );
    for result in results {
        let status = if result.ok() { "OK" } else { "FAIL" };
        println!("{status} {}", result.path.display());
        for diagnostic in &result.diagnostics {
            let location = diagnostic
                .position
                .as_ref()
                .map(|position| format!(":{}:{}", position.line, position.column))
                .unwrap_or_default();
            println!(
                "  {} {}{} {}",
                diagnostic.severity.as_str().to_ascii_uppercase(),
                diagnostic.code,
                location,
                diagnostic.message
            );
        }
    }
    let errors: usize = results.iter().map(ValidationResult::error_count).sum();
    let warnings: usize = results.iter().map(ValidationResult::warning_count).sum();
    println!(
        "\nValidated {} file(s): {} error(s), {} warning(s).",
        results.len(),
        errors,
        warnings
    );
}

pub fn print_json_results(results: &[ValidationResult], metadata: &RunMetadata) {
    let mut output = String::new();
    output.push_str("{\n");
    write!(
        output,
        "  \"metadata\": {{\n    \"tool\": {{\"name\": \"{}\", \"version\": \"{}\"}},\n    \"rule_catalog\": {{\"version\": \"{}\"}},\n    \"invocation\": {{\"timestamp_utc\": \"{}\", \"timestamp_epoch_seconds\": {}, \"backend\": \"{}\", \"strict\": {}, \"format\": \"{}\"}}\n  }},\n",
        json_escape(metadata.tool_name),
        json_escape(metadata.tool_version),
        json_escape(metadata.rule_catalog_version),
        json_escape(&metadata.timestamp_utc),
        metadata.timestamp_epoch_seconds,
        json_escape(metadata.backend),
        metadata.strict,
        json_escape(metadata.format),
    )
    .expect("write to String cannot fail");
    output.push_str("  \"results\": [\n");
    for (index, result) in results.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        write!(
            output,
            "    {{\n      \"path\": \"{}\",\n      \"ok\": {},\n      \"error_count\": {},\n      \"warning_count\": {},\n      \"diagnostics\": [",
            json_escape(&result.path.to_string_lossy()),
            result.ok(),
            result.error_count(),
            result.warning_count()
        )
        .expect("write to String cannot fail");
        if !result.diagnostics.is_empty() {
            output.push('\n');
        }
        for (diagnostic_index, diagnostic) in result.diagnostics.iter().enumerate() {
            if diagnostic_index > 0 {
                output.push_str(",\n");
            }
            write!(
                output,
                "        {{\"severity\": \"{}\", \"code\": \"{}\", \"message\": \"{}\", \"path\": \"{}\"",
                diagnostic.severity.as_str(),
                diagnostic.code,
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
            output.push('}');
        }
        if !result.diagnostics.is_empty() {
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
