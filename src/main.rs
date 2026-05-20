mod ast;
mod backend;
mod baseline;
mod config;
mod diag;
mod glob;
mod imports;
mod info;
mod junit;
mod lex;
mod library;
mod lsp;
mod manifest;
mod project;
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
use library::LibraryLoader;
use project::ProjectIndex;
use report::{
    print_json_results, print_junit_results, print_plain_results, print_sarif_results,
    print_text_results, RunMetadata,
};
use validate::{is_supported_model_path, validate_native};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Plain,
    Json,
    Sarif,
    Junit,
}

impl OutputFormat {
    fn as_str(self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Plain => "plain",
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
    library_path: Option<PathBuf>,
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
        Some("library-info") => run_library_info(&args[1..]),
        Some("lsp") => run_lsp(&args[1..]),
        Some("-V") | Some("--version") => {
            print_version();
            Ok(0)
        }
        Some("-h") | Some("--help") | None => {
            print_help();
            Ok(0)
        }
        Some(command) => Err(format!(
            "unknown command '{command}' (run `sysml-validate --help` for the subcommand list)"
        )),
    }
}

fn run_lsp(args: &[String]) -> Result<i32, String> {
    // `lsp` itself takes no operational flags today; we only accept
    // -h/--help. Anything else would otherwise be silently ignored
    // (the LSP loop reads stdin), which is a confusing UX.
    let mut show_help = false;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => show_help = true,
            other => {
                return Err(format!(
                    "unknown lsp option '{other}' (lsp takes no flags; it speaks LSP over stdin/stdout)"
                ));
            }
        }
    }
    if show_help {
        print_lsp_help();
        return Ok(0);
    }
    lsp::run().map(|_| 0)
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
    let manifest = manifest::discover(&working_directory)?;

    apply_config_defaults(&loaded, &mut args)?;

    let library = match &args.library_path {
        Some(path) => LibraryLoader::from_path(path)?,
        None => LibraryLoader::embedded(),
    };

    let baseline = match &args.baseline_path {
        Some(path) if path.is_file() => Some(Baseline::load(path)?),
        Some(_) => Some(Baseline::empty()), // --update-baseline on a fresh project
        None => None,
    };

    // Project-root resolution order:
    //   1. Sysand .project.json `root` field, if a manifest was found
    //   2. sysml-validate.toml `project_root` field
    //   3. Directory containing the discovered config file
    let project_root = manifest
        .source_root
        .clone()
        .or_else(|| loaded.config.project_root.clone())
        .or_else(|| {
            loaded
                .path
                .as_ref()
                .and_then(|p| p.parent().map(Path::to_path_buf))
        });
    let files = discover_files(&args.paths, &loaded.config, project_root.as_deref())?;
    if files.is_empty() {
        return Err("no .sysml or .kerml files found".to_string());
    }

    // Pre-pass: build a project-wide symbol table so cross-file imports
    // resolve under `--strict` (US-203) and so the specialization graph
    // can be cycle-checked (US-205 / SYSML220).
    let project_index = ProjectIndex::build(&files);

    let cycle_diagnostic = project_index
        .find_specialization_cycle()
        .map(|cycle| diag::Diagnostic::new(
            diag::Severity::Error,
            "SYSML220",
            format!(
                "Specialization graph contains a cycle: {}. Cycles break classification reasoning and are rejected by the OMG Pilot.",
                cycle.join(" :> ")
            ),
            files.first().map(PathBuf::as_path).unwrap_or(Path::new("project")),
            None,
        ));

    let timeout = Duration::from_secs(args.timeout_seconds);
    let mut results = Vec::new();
    for file in files {
        let result = match args.backend {
            BackendKind::Native => {
                validate_native(&file, args.strict, &loaded.config, &library, &project_index)
            }
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

    // SYSML220 is a project-level finding; attach to the first
    // ValidationResult so it surfaces in text / SARIF / JUnit paths.
    if let (Some(diagnostic), Some(first)) = (cycle_diagnostic, results.first_mut()) {
        first.diagnostics.push(diagnostic);
    }

    // When updating the baseline, accept everything: the whole point is
    // to enshrine the current state as the future reference.
    let exit_code = if args.update_baseline {
        0
    } else {
        compute_exit_code(&results, &args, baseline.as_ref(), project_root.as_deref())
    };

    let mut metadata = RunMetadata::capture(
        args.backend.as_str(),
        args.strict,
        args.fail_on_warning,
        args.format.as_str(),
        loaded.path.as_ref().map(|path| path.display().to_string()),
        args.baseline_path
            .as_ref()
            .map(|path| path.display().to_string()),
    );
    if manifest.manifest_path.is_some() {
        metadata.project_label = Some(manifest.display_label());
        metadata.project_manifest_path = manifest
            .manifest_path
            .as_ref()
            .map(|path| path.display().to_string());
    }

    match args.format {
        OutputFormat::Text => print_text_results(&results, &metadata, args.show_suppressed),
        OutputFormat::Plain => print_plain_results(&results, args.show_suppressed),
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
                    fs::write(path, &document).map_err(|error| {
                        format!("unable to write baseline '{}': {error}", path.display())
                    })?;
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
        OutputFormat::Plain | OutputFormat::Sarif | OutputFormat::Junit => {
            return Err("grammar-info supports only --format text|json".into());
        }
    }
    Ok(0)
}

fn run_library_info(args: &[String]) -> Result<i32, String> {
    let mut format = OutputFormat::Text;
    let mut library_path: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--format" => {
                index += 1;
                format = parse_format(args.get(index).ok_or("--format requires a value")?)?;
            }
            "--library-path" => {
                index += 1;
                let value = args.get(index).ok_or("--library-path requires a value")?;
                library_path = Some(PathBuf::from(value));
            }
            "-h" | "--help" => {
                print_library_help();
                return Ok(0);
            }
            option => return Err(format!("unknown library-info option '{option}'")),
        }
        index += 1;
    }

    let library = match library_path {
        Some(path) => LibraryLoader::from_path(&path)?,
        None => LibraryLoader::embedded(),
    };

    match format {
        OutputFormat::Text => {
            println!("Source: {}", library.source_description());
            println!("Files:  {}", library.file_count());
            println!("Declarations: {}", library.declaration_count());
            println!("Packages:");
            for package in library.package_names() {
                println!("  {package}");
            }
        }
        OutputFormat::Json => {
            use std::fmt::Write as _;
            let mut output = String::from("{\n");
            write!(
                output,
                "  \"source\": \"{}\",\n  \"files\": {},\n  \"declarations\": {},\n  \"packages\": [",
                report::json_escape(library.source_description()),
                library.file_count(),
                library.declaration_count()
            )
            .expect("write");
            let packages: Vec<&str> = library.package_names().collect();
            for (index, package) in packages.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                write!(output, "\"{}\"", report::json_escape(package)).expect("write");
            }
            output.push_str("]\n}");
            println!("{output}");
        }
        OutputFormat::Plain | OutputFormat::Sarif | OutputFormat::Junit => {
            return Err("library-info supports only --format text|json".into());
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
        OutputFormat::Plain | OutputFormat::Sarif | OutputFormat::Junit => {
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
    let mut library_path: Option<PathBuf> = None;
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
                timeout_seconds = value.parse::<u64>().map_err(|error| {
                    format!("--timeout must be a non-negative integer ({error})")
                })?;
            }
            "--config" => {
                index += 1;
                let value = args.get(index).ok_or("--config requires a value")?;
                config_discovery = ConfigDiscoveryArg::Explicit(PathBuf::from(value));
            }
            "--no-config" => {
                config_discovery = ConfigDiscoveryArg::Disabled;
            }
            "--library-path" => {
                index += 1;
                let value = args.get(index).ok_or("--library-path requires a value")?;
                library_path = Some(PathBuf::from(value));
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
        library_path,
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
        "plain" => Ok(OutputFormat::Plain),
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
                || include_patterns
                    .iter()
                    .any(|pattern| pattern.matches(&candidate));
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
    println!("Preflight validator for SysML v2 and KerML textual models.");
    println!();
    println!("USAGE:");
    println!("  sysml-validate <SUBCOMMAND> [OPTIONS]");
    println!("  sysml-validate --help | --version");
    println!();
    println!("SUBCOMMANDS:");
    println!("  validate       Validate one or more .sysml / .kerml files or directories.");
    println!("  grammar-info   Show the SysML v2 / KerML textual grammar references in use.");
    println!("  corpus-info    Show public model corpora useful for smoke-testing.");
    println!("  library-info   Show the embedded SysML v2 standard library inventory.");
    println!("  lsp            Run the Language Server (stdio JSON-RPC; no flags).");
    println!();
    println!("GLOBAL OPTIONS:");
    println!(
        "  -h, --help     Show this help. Append to any subcommand for subcommand-specific help."
    );
    println!("  -V, --version  Print version and exit.");
    println!();
    println!("EXAMPLES:");
    println!("  # Validate a single file (text output, exit 1 if errors)");
    println!("  sysml-validate validate model.sysml");
    println!();
    println!(
        "  # CI mode: SARIF 2.1.0 to stdout, dropped into GitHub Advanced Security / Iron Bank"
    );
    println!("  sysml-validate validate src --ci > findings.sarif");
    println!();
    println!("  # Seed a baseline for incremental adoption on an existing project");
    println!("  sysml-validate validate src --ci --baseline baseline.sarif --update-baseline");
    println!();
    println!("  # Show what's in the embedded standard library");
    println!("  sysml-validate library-info");
    println!();
    println!("Run `sysml-validate <SUBCOMMAND> --help` for the full option list for a subcommand.");
    println!("Full reference: docs/ in this repository, especially");
    println!("  PRD-government-readiness.md, SECURITY.md, OFFLINE.md, accessibility.md.");
}

fn print_version() {
    // One-line cargo-style. Consumers that want richer build metadata
    // (rule catalog version, backend identity, library release) get
    // it in the per-run metadata block emitted by `validate`.
    println!("sysml-validate {}", env!("CARGO_PKG_VERSION"));
}

fn print_lsp_help() {
    println!("Usage: sysml-validate lsp");
    println!();
    println!("Run the Language Server. Speaks LSP 3.x over stdin/stdout JSON-RPC.");
    println!("No flags. The server is launched by an editor client (VS Code, Neovim,");
    println!("Helix, etc.) — running it manually from a terminal will appear to hang");
    println!("because it is waiting for a JSON-RPC `initialize` request on stdin.");
    println!();
    println!("Capabilities:");
    println!("  textDocument/didOpen          parse + validate buffer, publish diagnostics");
    println!("  textDocument/didChange        full-text sync, re-validate, re-publish");
    println!("  textDocument/didClose        clear diagnostics");
    println!("  textDocument/hover            render the SYSMLxxx rule catalog entry");
    println!("                                under the cursor as markdown");
    println!("  textDocument/publishDiagnostics  push diagnostics to client");
    println!();
    println!("Example client config (Neovim with nvim-lspconfig):");
    println!("  require'lspconfig'.sysml_validate.setup{{");
    println!("    cmd = {{ 'sysml-validate', 'lsp' }},");
    println!("    filetypes = {{ 'sysml', 'kerml' }},");
    println!("  }}");
}

fn print_validate_help() {
    println!("Usage: sysml-validate validate <PATHS>... [OPTIONS]");
    println!();
    println!("Validate one or more .sysml / .kerml files or directories. Directories");
    println!("are walked recursively. Validation is project-aware: cross-file imports");
    println!("resolve through the project index and the embedded SysML v2 standard");
    println!("library.");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help                      Show this help and exit.");
    println!(
        "      --format <FORMAT>           Output format: text | plain | json | sarif | junit"
    );
    println!("                                  (default: text)");
    println!(
        "                                    text   human-readable with header + per-file groups"
    );
    println!(
        "                                    plain  GCC-style one diagnostic per line, ANSI-free"
    );
    println!("                                           (screen-reader / IDE / grep friendly)");
    println!("                                    json   legacy JSON with metadata block");
    println!("                                    sarif  SARIF 2.1.0 (GitHub Advanced Security,");
    println!("                                           SonarQube, Iron Bank, Azure DevOps)");
    println!(
        "                                    junit  Maven Surefire JUnit XML (Jenkins, GitLab)"
    );
    println!("      --ci                        Shortcut for `--format sarif`.");
    println!(
        "      --strict                    Warn on unresolved identifier references (SYSML040)."
    );
    println!("      --fail-on-warning           Exit 1 if any warning is produced.");
    println!("      --show-suppressed           Include suppressed diagnostics in text / JSON.");
    println!(
        "                                  Suppressed always appear in SARIF as suppressions[]."
    );
    println!("      --config <PATH>             Use an explicit sysml-validate.toml.");
    println!("      --no-config                 Skip config-file discovery entirely.");
    println!("      --library-path <DIR>        Override the embedded SysML v2 std library with");
    println!("                                  an on-disk copy (e.g., a pre-release library).");
    println!("      --baseline <PATH>           Classify findings against a prior SARIF run.");
    println!("                                  Unchanged + suppressed do not affect exit code.");
    println!(
        "      --update-baseline           Overwrite --baseline with the current run and exit 0."
    );
    println!(
        "                                  Requires `--format sarif` (or `--ci`) and `--baseline`."
    );
    println!("      --backend <KIND>            native (built-in, default) | official (delegate).");
    println!(
        "      --official-command <TPL>    Argv template for `--backend official`. `{{file}}` is"
    );
    println!(
        "                                  replaced with each model path. Tokenized with shell-"
    );
    println!(
        "                                  style quoting and invoked with positional argv — no"
    );
    println!(
        "                                  shell is spawned, so metacharacters survive only as"
    );
    println!("                                  literal argv content.");
    println!("      --timeout <SECONDS>         Kill the official backend if it exceeds this. The");
    println!("                                  child is terminated and SYSML904 is emitted.");
    println!("                                  Default: 60.");
    println!();
    println!("EXIT CODES:");
    println!("  0   No errors found (or `--update-baseline` accepted the current state).");
    println!("  1   Validation errors found (or warnings under `--fail-on-warning`).");
    println!("  2   CLI / config / backend setup error (the run never completed).");
    println!();
    println!("CONFIGURATION:");
    println!("  `sysml-validate.toml` in the working directory or any ancestor is loaded");
    println!("  automatically. CLI flags always override config values. Supported keys:");
    println!("  project_root, default_format, default_strict, default_fail_on_warning,");
    println!("  include / exclude glob patterns, [rules] per-code severity overrides");
    println!("  (error | warning | info | off). Unknown keys are rejected at parse time.");
    println!();
    println!("PROJECT MANIFEST:");
    println!("  A Sysand-compatible `.project.json` in the working directory or any");
    println!("  ancestor is discovered automatically. Its `root` field defines the");
    println!("  source root and takes precedence over `sysml-validate.toml`'s");
    println!("  `project_root`.");
    println!();
    println!("SUPPRESSIONS:");
    println!("  Suppress diagnostics inline with a line comment:");
    println!("    // sysml-validate: disable=SYSML041");
    println!("    // sysml-validate: disable-next-line=SYSML041,SYSML040");
    println!("    // sysml-validate: disable=all");
    println!("  Directives that don't match any diagnostic produce SYSML050;");
    println!("  invalid directive syntax produces SYSML060.");
    println!();
    println!("ENVIRONMENT:");
    println!("  NO_COLOR             The tool emits zero ANSI escape sequences in any");
    println!("                       format, so NO_COLOR is honored trivially.");
    println!("                       See docs/accessibility.md.");
    println!("  SOURCE_DATE_EPOCH    Build-time only (governs embedded build timestamps");
    println!("                       for reproducible builds). Not consulted at runtime.");
    println!();
    println!("EXAMPLES:");
    println!("  # Validate a single file (text output)");
    println!("  sysml-validate validate model.sysml");
    println!();
    println!("  # CI: SARIF 2.1.0 to stdout for GitHub Advanced Security upload");
    println!("  sysml-validate validate src --ci > findings.sarif");
    println!();
    println!("  # Strict gate: warn on unresolved identifiers and fail on warnings");
    println!("  sysml-validate validate src --strict --fail-on-warning");
    println!();
    println!("  # Plain output for screen readers / grep / IDE problem matchers");
    println!("  sysml-validate validate src --format plain");
    println!();
    println!("  # JUnit XML for Jenkins / GitLab test reporters");
    println!("  sysml-validate validate src --format junit > junit.xml");
    println!();
    println!("  # Baseline mode — seed once, then only NEW findings fail CI");
    println!("  sysml-validate validate src --ci --baseline baseline.sarif --update-baseline");
    println!("  sysml-validate validate src --ci --baseline baseline.sarif");
    println!();
    println!("  # Delegate to the official validator with a 2-minute hard timeout");
    println!("  sysml-validate validate model.sysml --backend official \\");
    println!("      --official-command \"sysml-validator --strict {{file}}\" --timeout 120");
    println!();
    println!("See `docs/PRD-government-readiness.md` for the full rule catalog and");
    println!("`docs/SECURITY.md` for release-verification recipes.");
}

fn print_grammar_help() {
    println!("Usage: sysml-validate grammar-info [--format text|json]");
    println!();
    println!("Show the SysML v2 / KerML textual grammar references in use:");
    println!("filenames (`SysML-textual-bnf.kebnf`, `KerML-textual-bnf.kebnf`), the");
    println!("OMG specification revisions they track, and the on-disk path under");
    println!("`vendor/sysml-v2-release/bnf/`.");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help              Show this help and exit.");
    println!("      --format <FORMAT>   text (default) | json");
}

fn print_corpus_help() {
    println!("Usage: sysml-validate corpus-info [--format text|json]");
    println!();
    println!("List public SysML v2 model corpora useful for smoke tests and");
    println!("differential testing: official examples / training / validation /");
    println!("library models, community-curated repositories, and OMG machine-");
    println!("readable bundles. Repo URLs and short descriptions only; no network");
    println!("calls are made by this subcommand.");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help              Show this help and exit.");
    println!("      --format <FORMAT>   text (default) | json");
}

fn print_library_help() {
    println!("Usage: sysml-validate library-info [--format text|json] [--library-path <DIR>]");
    println!();
    println!("Show the inventory of the SysML v2 standard library: source description");
    println!("(embedded OMG release tag, or on-disk path), file count, declaration");
    println!("count, and the full list of indexed package names. Useful for");
    println!("confirming what `:>` / `:>>` references resolve against under `--strict`.");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help              Show this help and exit.");
    println!("      --format <FORMAT>   text (default) | json");
    println!("      --library-path <DIR>");
    println!("                          Show the inventory of an on-disk library");
    println!("                          directory instead of the embedded one.");
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
        assert_eq!(parse_format("plain").unwrap(), OutputFormat::Plain);
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
