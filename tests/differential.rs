//! Differential test against the OMG SysML v2 corpus (US-207).
//!
//! This integration test runs `sysml-validate` against the OMG-curated
//! example and validation corpora and asserts that the count of
//! findings does not regress relative to the baseline recorded in
//! [`docs/differential-corpus-report.md`].
//!
//! Marked `#[ignore]` because it runs the full corpus (~150 files,
//! several seconds) and we don't want it slowing down the default
//! `cargo test` loop. Run explicitly:
//!
//! ```bash
//! cargo test --test differential -- --ignored
//! ```
//!
//! When intentionally changing counts (e.g., fixing a false positive
//! or adding a new rule), update the constants below AND
//! `docs/differential-corpus-report.md` in the same commit.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Baseline finding counts for the OMG **examples** corpus.
/// Update these together with docs/differential-corpus-report.md.
///
/// History:
///   v0.9.0  initial: 271 findings; demoted 212/213, fixed qualified-target FPs
///   v0.10.0 AST-aware name collection (Batch K) → 110/45/39 SYSML210/213/211
///   v0.11.0 AST-aware inherited-zone suppression (Batch L) → 213 drops 45→1
const EXAMPLES_BASELINE: &[(&str, usize)] = &[
    ("SYSML033", 6),
    ("SYSML041", 1),
    ("SYSML210", 110),
    ("SYSML211", 39),
    ("SYSML212", 1),
    ("SYSML213", 1),
    ("SYSML220", 1),
];

/// Baseline finding counts for the OMG **validation** corpus.
const VALIDATION_BASELINE: &[(&str, usize)] = &[("SYSML210", 31), ("SYSML211", 9), ("SYSML220", 1)];

#[test]
#[ignore = "runs the full OMG corpus; use cargo test --test differential -- --ignored"]
fn differential_examples_corpus() {
    let corpus = workspace_path("vendor/sysml-v2-release/sysml/src/examples");
    assert_corpus_matches_baseline("examples", &corpus, EXAMPLES_BASELINE);
}

#[test]
#[ignore = "runs the full OMG corpus; use cargo test --test differential -- --ignored"]
fn differential_validation_corpus() {
    let corpus = workspace_path("vendor/sysml-v2-release/sysml/src/validation");
    assert_corpus_matches_baseline("validation", &corpus, VALIDATION_BASELINE);
}

fn assert_corpus_matches_baseline(label: &str, corpus: &Path, baseline: &[(&str, usize)]) {
    assert!(
        corpus.is_dir(),
        "corpus directory missing: {} \
         (did you `git submodule update --init --recursive`?)",
        corpus.display()
    );

    let binary = workspace_path("target/release/sysml-validate.exe");
    let binary_unix = workspace_path("target/release/sysml-validate");
    let binary = if binary.exists() { binary } else { binary_unix };
    assert!(
        binary.exists(),
        "release binary missing at {}; run `cargo build --release` first",
        binary.display()
    );

    let output = Command::new(&binary)
        .arg("validate")
        .arg(corpus)
        .arg("--format")
        .arg("plain")
        .output()
        .expect("invoke sysml-validate");

    // Plain output goes to stdout, errors to stderr; both are line-
    // oriented. Concatenate for the regex sweep.
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&output.stderr));

    let counts = histogram_of_rule_codes(&text);
    let expected: HashMap<&str, usize> = baseline.iter().copied().collect();

    let mut drift = Vec::new();
    for (code, expected_count) in &expected {
        let actual = counts.get(*code).copied().unwrap_or(0);
        if actual != *expected_count {
            drift.push(format!("{code}: baseline={expected_count} actual={actual}"));
        }
    }
    for (code, actual) in &counts {
        if !expected.contains_key(code.as_str()) {
            drift.push(format!("{code}: baseline=0 actual={actual} (new)"));
        }
    }

    assert!(
        drift.is_empty(),
        "{label} corpus diverged from baseline:\n  {}\n\nIf this is \
         intentional, update EXAMPLES_BASELINE/VALIDATION_BASELINE in \
         tests/differential.rs AND docs/differential-corpus-report.md \
         in the same commit.",
        drift.join("\n  ")
    );
}

fn histogram_of_rule_codes(text: &str) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for line in text.lines() {
        // Look for "SYSMLxxx" tokens. Each diagnostic line has at most
        // one (in `--format plain` output).
        if let Some(start) = line.find("SYSML") {
            let bytes = line.as_bytes();
            let mut end = start + "SYSML".len();
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > start + "SYSML".len() {
                let code = &line[start..end];
                *counts.entry(code.to_string()).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn workspace_path(relative: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join(relative)
}
