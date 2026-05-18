use std::path::Path;

use crate::diag::{Diagnostic, Position, Severity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Identifier,
    Keyword,
    String,
    Symbol,
    Operator,
    Number,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
        }
    }
}

pub struct Scanner<'a> {
    path: &'a Path,
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Scanner<'a> {
    pub fn new(path: &'a Path, text: &str) -> Self {
        Self {
            path,
            chars: text.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            diagnostics: Vec::new(),
        }
    }

    pub fn scan(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
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

pub fn is_keyword(value: &str) -> bool {
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
