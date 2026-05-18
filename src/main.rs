mod backend;
mod baseline;
mod config;
mod diag;
mod glob;
mod info;
mod junit;
mod lex;
mod report;
mod rules;
mod sarif;
mod suppress;
mod validate;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use backend::{validate_official, DEFAULT_OFFICIAL_TIMEOUT_SECONDS};
use baseline::Baseline;
use config::{Config, ConfigDiscovery, LoadedConfig};
use glob::Pattern;
use info::{print_corpus_json, print_corpus_text, print_grammar_json, print_grammar_text};
use report::{
    print_json_results, print_junit_results, print_sarif_results, print_text_results, RunMetadata,
};
use validate::{is_supported_model_path, validate_native};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Json,
    Sarif,
    Junit,
}

impl OutputFormat {
    fn as_str(self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
            OutputFormat::Sarif => "sarif",
            OutputFormat::Junit => "junit",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BackendKind {
    Native,
    Official,
}

impl BackendKind {
    fn as_str(self) -> &'static str {
        match self {
            BackendKind::Native => "native",
            BackendKind::Official => "official",
        }
    }
}

#[derive(Debug)]
struct ValidateArgs {
    paths: Vec<PathBuf>,
    format: OutputFormat,
    strict: bool,
    fail_on_warning: bool,
    show_suppressed: bool,
    backend: BackendKind,
    official_command: Option<String>,
    timeout_seconds: u64,
    config_discovery: ConfigDiscoveryArg,
    baseline_path: Option<PathBuf>,
    update_baseline: bool,
    raw_command_line: String,
    cli_set_format: bool,
    cli_set_strict: bool,
    cli_set_fail_on_warning: bool,
}

#[derive(Clone, Debug)]
enum ConfigDiscoveryArg {
    Auto,
    Explicit(PathBuf),
    Disabled,
}

fn main() {
    let raw_args: Vec<String> = env::args().collect();
    let code = match run(&raw_args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            2
        }
    };
    std::process::exit(code);
}

fn run(raw_args: &[String]) -> Result<i32, String> {
    let args: Vec<String> = raw_args.iter().skip(1).cloned().collect();
    match args.first().map(String::as_str) {
        Some("validate") => run_validate(&args[1..], raw_args),
        Some("grammar-info") => run_grammar_info(&args[1..]),
        Some("corpus-info") => run_corpus_info(&args[1..]),
        Some("-h") | Some("--help") | None => {
            print_help();
            Ok(0)
        }
        Some(command) => Err(format!("unknown command '{command}'")),
    }
}

fn run_validate(args: &[String], raw_args: &[String]) -> Result<i32, String> {
    let mut args = parse_validate_args(args)?;
    args.raw_command_line = raw_args.join(" ");

    let working_directory = env::current_dir()
        .map_err(|error| format!("unable to determine current directory: {error}"))?;

    let discovery = match &args.config_discovery {
        ConfigDiscoveryArg::Auto => ConfigDiscovery::Auto,
        ConfigDiscoveryArg::Explicit(path) => ConfigDiscovery::Explicit(path),
        ConfigDiscoveryArg::Disabled => ConfigDiscovery::Disabled,
    };
    let loaded = config::load(&working_directory, discovery)?;

    apply_config_defaults(&loaded, &mut args)?;

    let baseline = match &args.baseline_path {
        Some(path) if path.is_file() => Some(Baseline::load(path)?),
        Some(_) => Some(Baseline::empty()), // --update-baseline on a fresh project
        None => None,
    };

    let project_root = loaded
        .config
        .project_root
        .clone()
        .or_else(|| loaded.path.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)));
    let files = discover_files(&args.paths, &loaded.config, project_root.as_deref())?;
    if files.is_empty() {
        return Err("no .sysml or .kerml files found".to_string());
    }

    let timeout = Duration::from_secs(args.timeout_seconds);
    let mut results = Vec::new();
    for file in files {
        let result = match args.backend {
            BackendKind::Native => validate_native(&file, args.strict, &loaded.config),
            BackendKind::Official => validate_official(
                &file,
                args.official_command
                    .as_deref()
                    .ok_or("--backend official requires --official-command")?,
                timeout,
            ),
        };
        results.push(result);
    }

    // When updating the baseline, accept everything: the whole point is
    // to enshrine the current state as the future reference.
    let exit_code = if args.update_baseline {
        0
    } else {
        compute_exit_code(&results, &args, baseline.as_ref(), project_root.as_deref())
    };

    let metadata = RunMetadata::capture(
        args.backend.as_str(),
        args.strict,
        args.fail_on_warning,
        args.format.as_str(),
        loaded
            .path
            .as_ref()
            .map(|path| path.display().to_string()),
        args.baseline_path
            .as_ref()
            .map(|path| path.display().to_string()),
    );

    match args.format {
        OutputFormat::Text => print_text_results(&results, &metadata, args.show_suppressed),
        OutputFormat::Json => print_json_results(&results, &metadata, args.show_suppressed),
        OutputFormat::Sarif => {
            let document = print_sarif_results(
                &results,
                &metadata,
                &args.raw_command_line,
                project_root.as_deref(),
                baseline.as_ref(),
                exit_code,
            );
            if args.update_baseline {
                if let Some(path) = &args.baseline_path {
                    fs::write(path, &document)
                        .map_err(|error| format!("unable to write baseline '{}': {error}", path.display()))?;
                }
            }
        }
        OutputFormat::Junit => print_junit_results(&results, args.fail_on_warning),
    }

    Ok(exit_code)
}

/// Exit code: 0 unless we have a build-failing diagnostic. Suppressed
/// diagnostics never fail; baseline-matched ones don't fail either when a
/// baseline is loaded.
fn compute_exit_code(
    results: &[diag::ValidationResult],
    args: &ValidateArgs,
    baseline: Option<&Baseline>,
    project_root: Option<&Path>,
) -> i32 {
    let any_new_error = results.iter().any(|result| {
        result.diagnostics.iter().any(|diagnostic| {
            if diagnostic.is_suppressed() || diagnostic.severity != diag::Severity::Error {
                return false;
            }
            match baseline {
                None => true,
                Some(baseline) => matches!(
                    baseline.classify(diagnostic.code, &diagnostic.fingerprint(project_root)),
                    baseline::BaselineState::New
                ),
            }
        })
    });

    let any_new_warning = args.fail_on_warning
        && results.iter().any(|result| {
            result.diagnostics.iter().any(|diagnostic| {
                if diagnostic.is_suppressed() || diagnostic.severity != diag::Severity::Warning {
                    return false;
                }
                match baseline {
                    None => true,
                    Some(baseline) => matches!(
                        baseline.classify(diagnostic.code, &diagnostic.fingerprint(project_root)),
                        baseline::BaselineState::New
                    ),
                }
            })
        });

    if any_new_error || any_new_warning {
        1
    } else {
        0
    }
}

fn apply_config_defaults(loaded: &LoadedConfig, args: &mut ValidateArgs) -> Result<(), String> {
    let cfg = &loaded.config;

    if !args.cli_set_format {
        if let Some(format_string) = cfg.default_format.as_deref() {
            args.format = parse_format(format_string)
                .map_err(|error| format!("invalid default_format in config: {error}"))?;
        }
    }
    if !args.cli_set_strict {
        if let Some(strict) = cfg.default_strict {
            args.strict = strict;
        }
    }
    if !args.cli_set_fail_on_warning {
        if let Some(fail_on_warning) = cfg.default_fail_on_warning {
            args.fail_on_warning = fail_on_warning;
        }
    }
    Ok(())
}

fn run_grammar_info(args: &[String]) -> Result<i32, String> {
    let mut format = OutputFormat::Text;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--format" => {
                index += 1;
                format = parse_format(args.get(index).ok_or("--format requires a value")?)?;
            }
            "-h" | "--help" => {
                print_grammar_help();
                return Ok(0);
            }
            option => return Err(format!("unknown grammar-info option '{option}'")),
        }
        index += 1;
    }

    match format {
        OutputFormat::Text => print_grammar_text(),
        OutputFormat::Json => print_grammar_json(),
        OutputFormat::Sarif | OutputFormat::Junit => {
            return Err("grammar-info supports only --format text|json".into());
        }
    }
    Ok(0)
}

fn run_corpus_info(args: &[String]) -> Result<i32, String> {
    let mut format = OutputFormat::Text;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--format" => {
                index += 1;
                format = parse_format(args.get(index).ok_or("--format requires a value")?)?;
            }
            "-h" | "--help" => {
                print_corpus_help();
                return Ok(0);
            }
            option => return Err(format!("unknown corpus-info option '{option}'")),
        }
        index += 1;
    }

    match format {
        OutputFormat::Text => print_corpus_text(),
        OutputFormat::Json => print_corpus_json(),
        OutputFormat::Sarif | OutputFormat::Junit => {
            return Err("corpus-info supports only --format text|json".into());
        }
    }
    Ok(0)
}

fn parse_validate_args(args: &[String]) -> Result<ValidateArgs, String> {
    let mut paths = Vec::new();
    let mut format = OutputFormat::Text;
    let mut cli_set_format = false;
    let mut strict = false;
    let mut cli_set_strict = false;
    let mut fail_on_warning = false;
    let mut cli_set_fail_on_warning = false;
    let mut show_suppressed = false;
    let mut backend = BackendKind::Native;
    let mut official_command = None;
    let mut timeout_seconds = DEFAULT_OFFICIAL_TIMEOUT_SECONDS;
    let mut config_discovery = ConfigDiscoveryArg::Auto;
    let mut baseline_path: Option<PathBuf> = None;
    let mut update_baseline = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--format" => {
                index += 1;
                format = parse_format(args.get(index).ok_or("--format requires a value")?)?;
                cli_set_format = true;
            }
            "--ci" => {
                format = OutputFormat::Sarif;
                cli_set_format = true;
            }
            "--strict" => {
                strict = true;
                cli_set_strict = true;
            }
            "--fail-on-warning" => {
                fail_on_warning = true;
                cli_set_fail_on_warning = true;
            }
            "--show-suppressed" => {
                show_suppressed = true;
            }
            "--backend" => {
                index += 1;
                backend = match args.get(index).map(String::as_str) {
                    Some("native") => BackendKind::Native,
                    Some("official") => BackendKind::Official,
                    Some(value) => return Err(format!("unsupported backend '{value}'")),
                    None => return Err("--backend requires a value".to_string()),
                };
            }
            "--official-command" => {
                index += 1;
                official_command = Some(
                    args.get(index)
                        .ok_or("--official-command requires a value")?
                        .to_string(),
                );
            }
            "--timeout" => {
                index += 1;
                let value = args.get(index).ok_or("--timeout requires a value")?;
                timeout_seconds = value
                    .parse::<u64>()
                    .map_err(|error| format!("--timeout must be a non-negative integer ({error})"))?;
            }
            "--config" => {
                index += 1;
                let value = args.get(index).ok_or("--config requires a value")?;
                config_discovery = ConfigDiscoveryArg::Explicit(PathBuf::from(value));
            }
            "--no-config" => {
                config_discovery = ConfigDiscoveryArg::Disabled;
            }
            "--baseline" => {
                index += 1;
                let value = args.get(index).ok_or("--baseline requires a value")?;
                baseline_path = Some(PathBuf::from(value));
            }
            "--update-baseline" => {
                update_baseline = true;
            }
            "-h" | "--help" => {
                print_validate_help();
                std::process::exit(0);
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown validate option '{value}'"))
            }
            value => paths.push(PathBuf::from(value)),
        }
        index += 1;
    }

    if paths.is_empty() {
        return Err("validate requires at least one file or directory".to_string());
    }
    if backend == BackendKind::Official {
        let command = official_command
            .as_ref()
            .ok_or("--backend official requires --official-command")?;
        if !command.contains("{file}") {
            return Err("--official-command must include a {file} placeholder".to_string());
        }
    }
    if update_baseline {
        if baseline_path.is_none() {
            return Err("--update-baseline requires --baseline <path>".to_string());
        }
        if format != OutputFormat::Sarif {
            return Err("--update-baseline requires --format sarif (or --ci)".to_string());
        }
    }

    Ok(ValidateArgs {
        paths,
        format,
        strict,
        fail_on_warning,
        show_suppressed,
        backend,
        official_command,
        timeout_seconds,
        config_discovery,
        baseline_path,
        update_baseline,
        raw_command_line: String::new(),
        cli_set_format,
        cli_set_strict,
        cli_set_fail_on_warning,
    })
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        "sarif" => Ok(OutputFormat::Sarif),
        "junit" => Ok(OutputFormat::Junit),
        _ => Err(format!("unsupported format '{value}'")),
    }
}

fn discover_files(
    paths: &[PathBuf],
    config: &Config,
    project_root: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for path in paths {
        let path = fs::canonicalize(path)
            .map_err(|error| format!("unable to resolve '{}': {error}", path.to_string_lossy()))?;
        if path.is_file() {
            if is_supported_model_path(&path) {
                files.push(path);
            }
        } else if path.is_dir() {
            collect_model_files(&path, &mut files)?;
        }
    }
    files.sort();

    let include_patterns: Vec<Pattern> = config.include.iter().map(|s| Pattern::new(s)).collect();
    let exclude_patterns: Vec<Pattern> = config.exclude.iter().map(|s| Pattern::new(s)).collect();
    if include_patterns.is_empty() && exclude_patterns.is_empty() {
        return Ok(files);
    }

    let filtered: Vec<PathBuf> = files
        .into_iter()
        .filter(|file| {
            let candidate = relativize(file, project_root);
            let included = include_patterns.is_empty()
                || include_patterns.iter().any(|pattern| pattern.matches(&candidate));
            let excluded = exclude_patterns
                .iter()
                .any(|pattern| pattern.matches(&candidate));
            included && !excluded
        })
        .collect();
    Ok(filtered)
}

fn relativize(file: &Path, project_root: Option<&Path>) -> String {
    let trimmed = match project_root {
        Some(root) => file.strip_prefix(root).unwrap_or(file),
        None => file,
    };
    trimmed.to_string_lossy().to_string()
}

fn collect_model_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|error| format!("unable to read '{}': {error}", dir.to_string_lossy()))?
    {
        let entry = entry.map_err(|error| format!("unable to read directory entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_model_files(&path, files)?;
        } else if is_supported_model_path(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn print_help() {
    println!("sysml-validate {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage:");
    println!("  sysml-validate validate <paths> [--format text|json|sarif|junit] [--ci] [--strict] [--fail-on-warning] [--show-suppressed]");
    println!("  sysml-validate validate <paths> --baseline <prior.sarif> [--update-baseline --ci]");
    println!("  sysml-validate validate <paths> --backend official --official-command <template> [--timeout <seconds>]");
    println!("  sysml-validate grammar-info [--format text|json]");
    println!("  sysml-validate corpus-info [--format text|json]");
}

fn print_validate_help() {
    println!("Usage: sysml-validate validate <paths> [options]");
    println!();
    println!("Options:");
    println!("  --format text|json|sarif|junit  output format (default: text)");
    println!("  --ci                            shortcut for --format sarif");
    println!("  --strict                        warn on unresolved identifiers");
    println!("  --fail-on-warning               exit 1 if any warning is produced");
    println!("  --show-suppressed               include suppressed diagnostics in text/JSON");
    println!("  --config <path>                 explicit config file");
    println!("  --no-config                     skip config-file discovery");
    println!("  --baseline <path>               classify findings against a prior SARIF run");
    println!("  --update-baseline               overwrite --baseline with the current run (--ci only)");
    println!("  --backend native|official");
    println!("  --official-command <argv template containing {{file}}>");
    println!("  --timeout <seconds>             official backend only (default 60)");
    println!();
    println!("Configuration is read from sysml-validate.toml in the working");
    println!("directory or any ancestor. CLI flags always override config values.");
    println!("Config supports: project_root, default_format, default_strict,");
    println!("default_fail_on_warning, [rules] severity overrides, include/exclude");
    println!("glob patterns.");
    println!();
    println!("--official-command is tokenized with shell-style quoting and");
    println!("invoked with positional argv. No shell process is spawned. If the");
    println!("child exceeds --timeout it is terminated and SYSML904 is emitted.");
    println!();
    println!("Suppress diagnostics inline with:");
    println!("  // sysml-validate: disable=SYSML041");
    println!("  // sysml-validate: disable-next-line=SYSML041,SYSML040");
    println!("  // sysml-validate: disable=all");
    println!();
    println!("Baseline mode classifies each finding as 'new' or 'unchanged'.");
    println!("Only 'new' findings affect the exit code; 'unchanged' findings");
    println!("are reported with baselineState=unchanged in SARIF.");
}

fn print_grammar_help() {
    println!("Usage: sysml-validate grammar-info [--format text|json]");
}

fn print_corpus_help() {
    println!("Usage: sysml-validate corpus-info [--format text|json]");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_results_propagate_through_text_output() {
        let metadata = RunMetadata::capture("native", false, false, "text", None, None);
        let results: Vec<diag::ValidationResult> = Vec::new();
        print_text_results(&results, &metadata, false);
    }

    #[test]
    fn parse_format_supports_all_formats() {
        assert_eq!(parse_format("sarif").unwrap(), OutputFormat::Sarif);
        assert_eq!(parse_format("junit").unwrap(), OutputFormat::Junit);
        assert_eq!(parse_format("text").unwrap(), OutputFormat::Text);
        assert_eq!(parse_format("json").unwrap(), OutputFormat::Json);
    }

    #[test]
    fn update_baseline_requires_sarif_format() {
        let result = parse_validate_args(&[
            "--update-baseline".to_string(),
            "--baseline".to_string(),
            "b.sarif".to_string(),
            "examples".to_string(),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--format sarif"));
    }

    #[test]
    fn update_baseline_requires_baseline_path() {
        let result = parse_validate_args(&[
            "--update-baseline".to_string(),
            "--ci".to_string(),
            "examples".to_string(),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--baseline"));
    }
}
