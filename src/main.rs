use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const RELEASE_REPOSITORY: &str = "https://github.com/Systems-Modeling/SysML-v2-Release";
const RELEASE_BRANCH: &str = "master";

struct PublicCorpus {
    name: &'static str,
    url: &'static str,
    license_note: &'static str,
    description: &'static str,
}

const PUBLIC_CORPORA: &[PublicCorpus] = &[
    PublicCorpus {
        name: "Systems-Modeling/SysML-v2-Release",
        url: "https://github.com/Systems-Modeling/SysML-v2-Release/tree/master/sysml/src",
        license_note: "EPL-2.0 for included models; see repository LICENSE.",
        description: "Official release examples, training models, validation models, and normative libraries.",
    },
    PublicCorpus {
        name: "GfSE/SysML-v2-Models",
        url: "https://github.com/GfSE/SysML-v2-Models",
        license_note: "BSD-3-Clause by default, with per-model overrides allowed.",
        description: "Community-curated collection of reusable SysML v2 textual models.",
    },
    PublicCorpus {
        name: "sensmetry/advent-of-sysml-v2",
        url: "https://github.com/sensmetry/advent-of-sysml-v2",
        license_note: "MIT; see repository LICENSE.",
        description: "Training-course examples organized by lesson, including model folders.",
    },
    PublicCorpus {
        name: "sensmetry/smart-home-hub-example",
        url: "https://github.com/sensmetry/smart-home-hub-example",
        license_note: "MIT; see repository LICENSE.",
        description: "Small complete SysML v2 project for smart-home hub architecture variants.",
    },
    PublicCorpus {
        name: "OMG SysML machine-readable files",
        url: "https://www.omg.org/spec/SysML/machine-readable",
        license_note: "OMG specification artifact licensing applies.",
        description: "Machine-readable SysML 2.0 artifacts, including the Simple Vehicle Model.",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Position {
    line: usize,
    column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Diagnostic {
    severity: Severity,
    code: &'static str,
    message: String,
    path: PathBuf,
    position: Option<Position>,
}

impl Diagnostic {
    fn new(
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
struct ValidationResult {
    path: PathBuf,
    diagnostics: Vec<Diagnostic>,
}

impl ValidationResult {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            diagnostics: Vec::new(),
        }
    }

    fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count()
    }

    fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Warning)
            .count()
    }

    fn ok(&self) -> bool {
        self.error_count() == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenKind {
    Identifier,
    Keyword,
    String,
    Symbol,
    Operator,
    Number,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Token {
    kind: TokenKind,
    value: String,
    line: usize,
    column: usize,
}

impl Token {
    fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BackendKind {
    Native,
    Official,
}

#[derive(Debug)]
struct ValidateArgs {
    paths: Vec<PathBuf>,
    format: OutputFormat,
    strict: bool,
    backend: BackendKind,
    official_command: Option<String>,
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

    let mut results = Vec::new();
    for file in files {
        let result = match args.backend {
            BackendKind::Native => validate_native(&file, args.strict),
            BackendKind::Official => validate_official(
                &file,
                args.official_command
                    .as_deref()
                    .ok_or("--backend official requires --official-command")?,
            ),
        };
        results.push(result);
    }

    match args.format {
        OutputFormat::Text => print_text_results(&results),
        OutputFormat::Json => print_json_results(&results),
    }

    Ok(if results.iter().any(|result| !result.ok()) {
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
        OutputFormat::Text => {
            println!("Repository: {RELEASE_REPOSITORY}");
            println!("Branch: {RELEASE_BRANCH}");
            println!("Model projects: sysml, kerml, sysml.library");
            println!("Grammar references:");
            println!("  sysml_textual: bnf/SysML-textual-bnf.kebnf");
            println!("    https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/master/bnf/SysML-textual-bnf.kebnf");
            println!("  kerml_textual: bnf/KerML-textual-bnf.kebnf");
            println!("    https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/master/bnf/KerML-textual-bnf.kebnf");
        }
        OutputFormat::Json => {
            println!(
                "{{\n  \"repository\": \"{}\",\n  \"branch\": \"{}\",\n  \"model_projects\": [\"sysml\", \"kerml\", \"sysml.library\"],\n  \"grammars\": {{\n    \"sysml_textual\": {{\n      \"path\": \"bnf/SysML-textual-bnf.kebnf\",\n      \"url\": \"https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/master/bnf/SysML-textual-bnf.kebnf\"\n    }},\n    \"kerml_textual\": {{\n      \"path\": \"bnf/KerML-textual-bnf.kebnf\",\n      \"url\": \"https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/master/bnf/KerML-textual-bnf.kebnf\"\n    }}\n  }}\n}}",
                json_escape(RELEASE_REPOSITORY),
                json_escape(RELEASE_BRANCH)
            );
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
        OutputFormat::Text => {
            println!("Public SysML v2 model corpora:");
            for corpus in PUBLIC_CORPORA {
                println!("  {}", corpus.name);
                println!("    {}", corpus.url);
                println!("    {}", corpus.description);
                println!("    {}", corpus.license_note);
            }
        }
        OutputFormat::Json => {
            let mut output = String::from("{\n  \"corpora\": [\n");
            for (index, corpus) in PUBLIC_CORPORA.iter().enumerate() {
                if index > 0 {
                    output.push_str(",\n");
                }
                write!(
                    output,
                    "    {{\"name\": \"{}\", \"url\": \"{}\", \"description\": \"{}\", \"license_note\": \"{}\"}}",
                    json_escape(corpus.name),
                    json_escape(corpus.url),
                    json_escape(corpus.description),
                    json_escape(corpus.license_note)
                )
                .expect("write to String cannot fail");
            }
            output.push_str("\n  ]\n}");
            println!("{output}");
        }
    }
    Ok(0)
}

fn parse_validate_args(args: &[String]) -> Result<ValidateArgs, String> {
    let mut paths = Vec::new();
    let mut format = OutputFormat::Text;
    let mut strict = false;
    let mut backend = BackendKind::Native;
    let mut official_command = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--format" => {
                index += 1;
                format = parse_format(args.get(index).ok_or("--format requires a value")?)?;
            }
            "--strict" => strict = true,
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
        backend,
        official_command,
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

fn is_supported_model_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str).map(str::to_ascii_lowercase),
        Some(extension) if extension == "sysml" || extension == "kerml"
    )
}

fn validate_native(path: &Path, strict: bool) -> ValidationResult {
    let mut result = ValidationResult::new(path);
    if !is_supported_model_path(path) {
        result.diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML010",
            "Unsupported file extension. Expected .sysml or .kerml.",
            path,
            None,
        ));
        return result;
    }

    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            result.diagnostics.push(Diagnostic::new(
                Severity::Error,
                "SYSML012",
                format!("Unable to read file as UTF-8 text: {error}"),
                path,
                None,
            ));
            return result;
        }
    };

    let (tokens, scan_diagnostics) = Scanner::new(path, &text).scan();
    result.diagnostics.extend(scan_diagnostics);
    result
        .diagnostics
        .extend(validate_balanced_delimiters(path, &tokens));
    result
        .diagnostics
        .extend(validate_statement_shapes(path, &tokens));
    result
        .diagnostics
        .extend(validate_duplicate_scope_members(path, &tokens));
    if strict {
        result
            .diagnostics
            .extend(validate_reference_candidates(path, &tokens));
    }
    result
}

fn validate_official(path: &Path, command_template: &str) -> ValidationResult {
    let mut result = ValidationResult::new(path);
    let command = command_template.replace("{file}", &path.to_string_lossy());
    let output = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    };

    match output {
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

struct Scanner<'a> {
    path: &'a Path,
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Scanner<'a> {
    fn new(path: &'a Path, text: &str) -> Self {
        Self {
            path,
            chars: text.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            diagnostics: Vec::new(),
        }
    }

    fn scan(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        let mut tokens = Vec::new();
        while !self.at_end() {
            let character = self.peek();
            match character {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => self.advance_line(),
                '/' if self.peek_next() == '/' => self.scan_line_comment(),
                '/' if self.peek_next() == '*' => self.scan_block_comment(),
                '"' | '\'' => tokens.push(self.scan_quoted()),
                c if c.is_ascii_alphabetic() || c == '_' => tokens.push(self.scan_identifier()),
                c if c.is_ascii_digit() => tokens.push(self.scan_number()),
                c if c.is_control() => {
                    self.diagnostics.push(Diagnostic::new(
                        Severity::Error,
                        "SYSML001",
                        "Invalid control character in source text.",
                        self.path,
                        Some(Position {
                            line: self.line,
                            column: self.column,
                        }),
                    ));
                    self.advance();
                }
                _ => tokens.push(self.scan_symbol_or_operator()),
            }
        }
        (tokens, self.diagnostics)
    }

    fn scan_identifier(&mut self) -> Token {
        let line = self.line;
        let column = self.column;
        let mut value = String::new();
        while !self.at_end() {
            let character = self.peek();
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                value.push(self.advance());
            } else {
                break;
            }
        }
        let kind = if is_keyword(&value) {
            TokenKind::Keyword
        } else {
            TokenKind::Identifier
        };
        Token {
            kind,
            value,
            line,
            column,
        }
    }

    fn scan_number(&mut self) -> Token {
        let line = self.line;
        let column = self.column;
        let mut value = String::new();
        while !self.at_end() {
            let character = self.peek();
            if character.is_ascii_digit() || character == '.' {
                value.push(self.advance());
            } else {
                break;
            }
        }
        Token {
            kind: TokenKind::Number,
            value,
            line,
            column,
        }
    }

    fn scan_quoted(&mut self) -> Token {
        let quote = self.peek();
        let line = self.line;
        let column = self.column;
        let mut value = String::new();
        value.push(self.advance());
        let mut escaped = false;
        while !self.at_end() {
            let character = self.advance();
            value.push(character);
            if character == '\n' {
                self.line += 1;
                self.column = 1;
            }
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == quote {
                return Token {
                    kind: TokenKind::String,
                    value,
                    line,
                    column,
                };
            }
        }
        self.diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML002",
            "Unterminated string literal.",
            self.path,
            Some(Position { line, column }),
        ));
        Token {
            kind: TokenKind::String,
            value,
            line,
            column,
        }
    }

    fn scan_line_comment(&mut self) {
        while !self.at_end() && self.peek() != '\n' {
            self.advance();
        }
    }

    fn scan_block_comment(&mut self) {
        let line = self.line;
        let column = self.column;
        self.advance();
        self.advance();
        while !self.at_end() {
            if self.peek() == '*' && self.peek_next() == '/' {
                self.advance();
                self.advance();
                return;
            }
            if self.peek() == '\n' {
                self.advance_line();
            } else {
                self.advance();
            }
        }
        self.diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML003",
            "Unterminated block comment.",
            self.path,
            Some(Position { line, column }),
        ));
    }

    fn scan_symbol_or_operator(&mut self) -> Token {
        let line = self.line;
        let column = self.column;
        let three = self.peek_string(3);
        if matches!(three.as_str(), ":>>" | "::>") {
            self.advance();
            self.advance();
            self.advance();
            return Token {
                kind: TokenKind::Operator,
                value: three,
                line,
                column,
            };
        }
        let two = self.peek_string(2);
        if matches!(two.as_str(), ":>" | "::" | ":=" | "=>" | "->" | "..") {
            self.advance();
            self.advance();
            return Token {
                kind: TokenKind::Operator,
                value: two,
                line,
                column,
            };
        }
        let value = self.advance().to_string();
        let kind = if "{}()[];,=<>.*~".contains(value.as_str()) {
            TokenKind::Symbol
        } else {
            TokenKind::Operator
        };
        Token {
            kind,
            value,
            line,
            column,
        }
    }

    fn peek_string(&self, count: usize) -> String {
        self.chars
            .iter()
            .skip(self.index)
            .take(count)
            .collect::<String>()
    }

    fn peek(&self) -> char {
        self.chars[self.index]
    }

    fn peek_next(&self) -> char {
        self.chars.get(self.index + 1).copied().unwrap_or('\0')
    }

    fn advance(&mut self) -> char {
        let character = self.chars[self.index];
        self.index += 1;
        self.column += 1;
        character
    }

    fn advance_line(&mut self) {
        self.index += 1;
        self.line += 1;
        self.column = 1;
    }

    fn at_end(&self) -> bool {
        self.index >= self.chars.len()
    }
}

fn validate_balanced_delimiters(path: &Path, tokens: &[Token]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut stack: Vec<&Token> = Vec::new();
    for token in tokens {
        if matches!(token.value.as_str(), "{" | "(" | "[") {
            stack.push(token);
        } else if matches!(token.value.as_str(), "}" | ")" | "]") {
            match stack.last() {
                Some(open) if expected_close(open.value.as_str()) == Some(token.value.as_str()) => {
                    stack.pop();
                }
                _ => diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "SYSML020",
                    format!("Unmatched closing delimiter '{}'.", token.value),
                    path,
                    Some(token.position()),
                )),
            }
        }
    }
    for token in stack.into_iter().rev() {
        diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML021",
            format!(
                "Unclosed delimiter '{}', expected '{}'.",
                token.value,
                expected_close(token.value.as_str()).unwrap_or("?")
            ),
            path,
            Some(token.position()),
        ));
    }
    diagnostics
}

fn validate_statement_shapes(path: &Path, tokens: &[Token]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.value == "package" {
            diagnostics.extend(expect_name_and_terminator(path, tokens, index, "package"));
        } else if token.value == "library" {
            if tokens.get(index + 1).map(|token| token.value.as_str()) != Some("package") {
                diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "SYSML030",
                    "Expected 'package' after 'library'.",
                    path,
                    Some(token.position()),
                ));
            }
        } else if token.value == "import" {
            diagnostics.extend(expect_terminator(path, tokens, index, "import declaration"));
        } else if token.value == "alias" {
            diagnostics.extend(expect_terminator(path, tokens, index, "alias declaration"));
            if !contains_before_end(tokens, index, "for") {
                diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "SYSML031",
                    "Alias declaration must include 'for'.",
                    path,
                    Some(token.position()),
                ));
            }
        } else if token.value == "dependency" {
            diagnostics.extend(expect_terminator(path, tokens, index, "dependency"));
            if !contains_before_end(tokens, index, "to") {
                diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "SYSML032",
                    "Dependency declaration must include a supplier after 'to'.",
                    path,
                    Some(token.position()),
                ));
            }
        } else if is_definition_keyword(&token.value)
            && tokens.get(index + 1).map(|token| token.value.as_str()) == Some("def")
        {
            diagnostics.extend(expect_name_and_terminator(
                path,
                tokens,
                index + 1,
                &format!("{} definition", token.value),
            ));
        } else if is_usage_keyword(&token.value) {
            if previous_value(tokens, index) == Some("def") {
                continue;
            }
            if is_connector_short_form(&token.value) {
                diagnostics.extend(expect_terminator(
                    path,
                    tokens,
                    index,
                    &format!("{} usage", token.value),
                ));
                continue;
            }
            match tokens.get(index + 1) {
                None => diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    "SYSML033",
                    format!(
                        "Expected a declared name or specialization after '{}'.",
                        token.value
                    ),
                    path,
                    Some(token.position()),
                )),
                Some(next) if next.value == ";" || next.value == "{" => {
                    diagnostics.push(Diagnostic::new(
                        Severity::Error,
                        "SYSML033",
                        format!(
                            "Expected a declared name or specialization after '{}'.",
                            token.value
                        ),
                        path,
                        Some(token.position()),
                    ))
                }
                Some(_) => diagnostics.extend(expect_terminator(
                    path,
                    tokens,
                    index,
                    &format!("{} usage", token.value),
                )),
            }
        }
    }
    diagnostics
}

fn validate_duplicate_scope_members(path: &Path, tokens: &[Token]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut scopes: Vec<HashSet<String>> = vec![HashSet::new()];
    let mut pending_name: Option<(String, Token)> = None;
    for (index, token) in tokens.iter().enumerate() {
        match token.value.as_str() {
            "{" => {
                scopes.push(HashSet::new());
                if let Some((name, name_token)) = pending_name.take() {
                    let scope_index = scopes.len().saturating_sub(2);
                    record_name(
                        path,
                        &mut scopes[scope_index],
                        &name,
                        &name_token,
                        &mut diagnostics,
                    );
                }
            }
            "}" => {
                if scopes.len() > 1 {
                    scopes.pop();
                }
            }
            ";" => {
                if let Some((name, name_token)) = pending_name.take() {
                    let last = scopes.len() - 1;
                    record_name(
                        path,
                        &mut scopes[last],
                        &name,
                        &name_token,
                        &mut diagnostics,
                    );
                }
            }
            _ if starts_named_member(tokens, index) => {
                if let Some(name) = declared_name_after(tokens, index) {
                    pending_name = Some((name.value.clone(), name.clone()));
                }
            }
            _ => {}
        }
    }
    diagnostics
}

fn validate_reference_candidates(path: &Path, tokens: &[Token]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let declared: HashSet<String> = tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.kind == TokenKind::Identifier && is_declaration_name(tokens, *index)
        })
        .map(|(_, token)| token.value.clone())
        .collect();
    let reference_markers = [
        "for",
        "to",
        "from",
        ":>",
        "specializes",
        "subsets",
        "references",
        "redefines",
    ];
    for window in tokens.windows(2) {
        let marker = &window[0];
        let candidate = &window[1];
        if reference_markers.contains(&marker.value.as_str())
            && candidate.kind == TokenKind::Identifier
            && !declared.contains(&candidate.value)
        {
            diagnostics.push(Diagnostic::new(
                Severity::Warning,
                "SYSML040",
                format!(
                    "Reference '{}' is not declared in this file. It may resolve from imports or libraries.",
                    candidate.value
                ),
                path,
                Some(candidate.position()),
            ));
        }
    }
    diagnostics
}

fn record_name(
    path: &Path,
    scope: &mut HashSet<String>,
    name: &str,
    token: &Token,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if scope.contains(name) {
        diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML041",
            format!("Duplicate member name '{name}' in the same lexical scope."),
            path,
            Some(token.position()),
        ));
    }
    scope.insert(name.to_string());
}

fn expect_name_and_terminator(
    path: &Path,
    tokens: &[Token],
    index: usize,
    statement: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let name_index = index + 1;
    if tokens
        .get(name_index)
        .map(|token| token.value == ";" || token.value == "{")
        .unwrap_or(true)
    {
        diagnostics.push(Diagnostic::new(
            Severity::Error,
            "SYSML034",
            format!("Expected a name for {statement}."),
            path,
            tokens.get(index).map(Token::position),
        ));
    }
    diagnostics.extend(expect_terminator(path, tokens, index, statement));
    diagnostics
}

fn expect_terminator(
    path: &Path,
    tokens: &[Token],
    index: usize,
    statement: &str,
) -> Vec<Diagnostic> {
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut cursor = index + 1;
    while cursor < tokens.len() {
        match tokens[cursor].value.as_str() {
            "{" if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 => return Vec::new(),
            "{" => brace_depth += 1,
            "(" => paren_depth += 1,
            "[" => bracket_depth += 1,
            "}" if brace_depth > 0 => brace_depth -= 1,
            ")" if paren_depth > 0 => paren_depth -= 1,
            "]" if bracket_depth > 0 => bracket_depth -= 1,
            ";" if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 => return Vec::new(),
            _ => {}
        }
        cursor += 1;
    }
    vec![Diagnostic::new(
        Severity::Error,
        "SYSML035",
        format!("Expected ';' or '{{' to terminate {statement}."),
        path,
        tokens.get(index).map(Token::position),
    )]
}

fn starts_named_member(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token.value == "package"
        || (is_definition_keyword(&token.value) && next_value(tokens, index) == Some("def"))
        || (is_usage_keyword(&token.value) && previous_value(tokens, index) != Some("def"))
}

fn declared_name_after(tokens: &[Token], index: usize) -> Option<&Token> {
    let mut cursor = index + 1;
    if tokens.get(cursor).map(|token| token.value.as_str()) == Some("def") {
        cursor += 1;
    }
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.kind == TokenKind::Identifier {
            return Some(token);
        }
        if token.value == ";" || token.value == "{" {
            return None;
        }
        cursor += 1;
    }
    None
}

fn is_declaration_name(tokens: &[Token], index: usize) -> bool {
    let start = index.saturating_sub(3);
    for cursor in start..index {
        if starts_named_member(tokens, cursor) {
            if let Some(name) = declared_name_after(tokens, cursor) {
                if name == &tokens[index] {
                    return true;
                }
            }
        }
    }
    false
}

fn contains_before_end(tokens: &[Token], index: usize, value: &str) -> bool {
    let mut cursor = index + 1;
    while cursor < tokens.len() && tokens[cursor].value != ";" && tokens[cursor].value != "{" {
        if tokens[cursor].value == value {
            return true;
        }
        cursor += 1;
    }
    false
}

fn expected_close(value: &str) -> Option<&'static str> {
    match value {
        "{" => Some("}"),
        "(" => Some(")"),
        "[" => Some("]"),
        _ => None,
    }
}

fn next_value(tokens: &[Token], index: usize) -> Option<&str> {
    tokens.get(index + 1).map(|token| token.value.as_str())
}

fn previous_value(tokens: &[Token], index: usize) -> Option<&str> {
    index
        .checked_sub(1)
        .and_then(|previous| tokens.get(previous))
        .map(|token| token.value.as_str())
}

fn is_connector_short_form(value: &str) -> bool {
    matches!(
        value,
        "bind" | "connect" | "satisfy" | "verify" | "include" | "perform" | "assert"
    )
}

fn is_definition_keyword(value: &str) -> bool {
    matches!(
        value,
        "attribute"
            | "allocation"
            | "analysis"
            | "action"
            | "calc"
            | "case"
            | "concern"
            | "connection"
            | "constraint"
            | "enum"
            | "flow"
            | "interface"
            | "item"
            | "occurrence"
            | "part"
            | "port"
            | "rendering"
            | "requirement"
            | "state"
            | "use"
            | "verification"
            | "view"
            | "viewpoint"
    )
}

fn is_usage_keyword(value: &str) -> bool {
    matches!(
        value,
        "action"
            | "allocation"
            | "assert"
            | "attribute"
            | "bind"
            | "case"
            | "concern"
            | "connect"
            | "connection"
            | "constraint"
            | "flow"
            | "include"
            | "interface"
            | "item"
            | "occurrence"
            | "part"
            | "perform"
            | "port"
            | "ref"
            | "requirement"
            | "satisfy"
            | "state"
            | "use"
            | "verify"
            | "view"
    )
}

fn is_keyword(value: &str) -> bool {
    matches!(
        value,
        "about"
            | "abstract"
            | "accept"
            | "action"
            | "actor"
            | "after"
            | "alias"
            | "all"
            | "allocate"
            | "allocation"
            | "analysis"
            | "and"
            | "as"
            | "assert"
            | "assign"
            | "assume"
            | "at"
            | "attribute"
            | "bind"
            | "binding"
            | "by"
            | "calc"
            | "case"
            | "comment"
            | "concern"
            | "connect"
            | "connection"
            | "constant"
            | "constraint"
            | "crosses"
            | "decide"
            | "def"
            | "dependency"
            | "doc"
            | "enum"
            | "event"
            | "flow"
            | "from"
            | "import"
            | "in"
            | "inout"
            | "interface"
            | "item"
            | "library"
            | "package"
            | "part"
            | "perform"
            | "port"
            | "private"
            | "protected"
            | "public"
            | "references"
            | "ref"
            | "redefines"
            | "requirement"
            | "satisfy"
            | "specializes"
            | "state"
            | "subsets"
            | "to"
            | "use"
            | "view"
            | "viewpoint"
    )
}

fn print_text_results(results: &[ValidationResult]) {
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

fn print_json_results(results: &[ValidationResult]) {
    let mut output = String::new();
    output.push_str("{\n  \"results\": [\n");
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

fn json_escape(value: &str) -> String {
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

fn print_help() {
    println!("sysml-validate-rs");
    println!();
    println!("Usage:");
    println!("  sysml-validate-rs validate <paths> [--format text|json] [--strict]");
    println!(
        "  sysml-validate-rs validate <paths> --backend official --official-command <template>"
    );
    println!("  sysml-validate-rs grammar-info [--format text|json]");
    println!("  sysml-validate-rs corpus-info [--format text|json]");
}

fn print_validate_help() {
    println!("Usage: sysml-validate-rs validate <paths> [options]");
    println!();
    println!("Options:");
    println!("  --format text|json");
    println!("  --strict");
    println!("  --backend native|official");
    println!("  --official-command <template containing {{file}}>");
}

fn print_grammar_help() {
    println!("Usage: sysml-validate-rs grammar-info [--format text|json]");
}

fn print_corpus_help() {
    println!("Usage: sysml-validate-rs corpus-info [--format text|json]");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn accepts_basic_package() {
        let result = validate_temp(
            "package Vehicle { part def Engine; part engine : Engine; attribute mass; }",
            ".sysml",
            false,
        );
        assert!(result.ok(), "{:?}", result.diagnostics);
    }

    #[test]
    fn reports_unbalanced_delimiter() {
        let result = validate_temp("package Vehicle { part def Engine;", ".sysml", false);
        assert!(!result.ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SYSML021"));
    }

    #[test]
    fn reports_missing_alias_for() {
        let result = validate_temp("package P { alias E Engine; }", ".sysml", false);
        assert!(!result.ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SYSML031"));
    }

    #[test]
    fn reports_duplicate_member_in_scope() {
        let result = validate_temp(
            "package P { part def Engine; part def Engine; }",
            ".sysml",
            false,
        );
        assert!(!result.ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SYSML041"));
    }

    #[test]
    fn reports_strict_reference_warning() {
        let result = validate_temp("package P { part engine :> Missing; }", ".sysml", true);
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SYSML040"));
    }

    fn validate_temp(text: &str, suffix: &str, strict: bool) -> ValidationResult {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is before UNIX_EPOCH")
            .as_nanos();
        let dir = env::temp_dir().join(format!("sysml_validate_rs_{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join(format!("model{suffix}"));
        fs::write(&path, text).expect("write temp model");
        let result = validate_native(&path, strict);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
        result
    }
}
