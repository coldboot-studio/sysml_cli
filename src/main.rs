mod backend;
mod diag;
mod info;
mod lex;
mod report;
mod validate;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use backend::{validate_official, DEFAULT_OFFICIAL_TIMEOUT_SECONDS};
use info::{print_corpus_json, print_corpus_text, print_grammar_json, print_grammar_text};
use report::{print_json_results, print_text_results, RunMetadata};
use validate::{is_supported_model_path, validate_native};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    fn as_str(self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
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
    backend: BackendKind,
    official_command: Option<String>,
    timeout_seconds: u64,
}

fn main() {
    let code = match run(env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            2
        }
    };
    std::process::exit(code);
}

fn run(args: Vec<String>) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        Some("validate") => run_validate(&args[1..]),
        Some("grammar-info") => run_grammar_info(&args[1..]),
        Some("corpus-info") => run_corpus_info(&args[1..]),
        Some("-h") | Some("--help") | None => {
            print_help();
            Ok(0)
        }
        Some(command) => Err(format!("unknown command '{command}'")),
    }
}

fn run_validate(args: &[String]) -> Result<i32, String> {
    let args = parse_validate_args(args)?;
    let files = discover_files(&args.paths)?;
    if files.is_empty() {
        return Err("no .sysml or .kerml files found".to_string());
    }

    let timeout = Duration::from_secs(args.timeout_seconds);
    let mut results = Vec::new();
    for file in files {
        let result = match args.backend {
            BackendKind::Native => validate_native(&file, args.strict),
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

    let metadata = RunMetadata::capture(
        args.backend.as_str(),
        args.strict,
        args.fail_on_warning,
        args.format.as_str(),
    );
    match args.format {
        OutputFormat::Text => print_text_results(&results, &metadata),
        OutputFormat::Json => print_json_results(&results, &metadata),
    }

    let any_errors = results.iter().any(|result| !result.ok());
    let any_warnings = results.iter().any(|result| result.warning_count() > 0);
    Ok(if any_errors || (args.fail_on_warning && any_warnings) {
        1
    } else {
        0
    })
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
    }
    Ok(0)
}

fn parse_validate_args(args: &[String]) -> Result<ValidateArgs, String> {
    let mut paths = Vec::new();
    let mut format = OutputFormat::Text;
    let mut strict = false;
    let mut fail_on_warning = false;
    let mut backend = BackendKind::Native;
    let mut official_command = None;
    let mut timeout_seconds = DEFAULT_OFFICIAL_TIMEOUT_SECONDS;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--format" => {
                index += 1;
                format = parse_format(args.get(index).ok_or("--format requires a value")?)?;
            }
            "--strict" => strict = true,
            "--fail-on-warning" => fail_on_warning = true,
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

    Ok(ValidateArgs {
        paths,
        format,
        strict,
        fail_on_warning,
        backend,
        official_command,
        timeout_seconds,
    })
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err(format!("unsupported format '{value}'")),
    }
}

fn discover_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
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
    Ok(files)
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
    println!("  sysml-validate validate <paths> [--format text|json] [--strict]");
    println!(
        "  sysml-validate validate <paths> --backend official --official-command <template> [--timeout <seconds>]"
    );
    println!("  sysml-validate grammar-info [--format text|json]");
    println!("  sysml-validate corpus-info [--format text|json]");
}

fn print_validate_help() {
    println!("Usage: sysml-validate validate <paths> [options]");
    println!();
    println!("Options:");
    println!("  --format text|json");
    println!("  --strict                        warn on unresolved identifiers");
    println!("  --fail-on-warning               exit 1 if any warning is produced");
    println!("  --backend native|official");
    println!("  --official-command <argv template containing {{file}}>");
    println!("  --timeout <seconds>             official backend only (default 60)");
    println!();
    println!("--official-command is tokenized with shell-style quoting (single and");
    println!("double quotes, backslash escapes inside double quotes) and invoked");
    println!("with positional argv. No shell process is spawned. If the child");
    println!("exceeds --timeout it is terminated and a SYSML904 diagnostic is");
    println!("emitted.");
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
        // Sanity check that the binary's wiring compiles together.
        let metadata = RunMetadata::capture("native", false, false, "text");
        let results: Vec<diag::ValidationResult> = Vec::new();
        print_text_results(&results, &metadata);
    }
}
