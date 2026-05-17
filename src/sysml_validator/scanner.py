from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto
from pathlib import Path

from .models import Diagnostic, Position, Severity


class TokenKind(Enum):
    IDENTIFIER = auto()
    KEYWORD = auto()
    STRING = auto()
    COMMENT = auto()
    SYMBOL = auto()
    OPERATOR = auto()
    NUMBER = auto()


@dataclass(frozen=True)
class Token:
    kind: TokenKind
    value: str
    line: int
    column: int

    @property
    def position(self) -> Position:
        return Position(self.line, self.column)


KEYWORDS = {
    "about",
    "abstract",
    "accept",
    "action",
    "actor",
    "after",
    "alias",
    "all",
    "allocate",
    "allocation",
    "analysis",
    "and",
    "as",
    "assert",
    "assign",
    "assume",
    "at",
    "attribute",
    "bind",
    "binding",
    "by",
    "calc",
    "case",
    "comment",
    "concern",
    "connect",
    "connection",
    "constant",
    "constraint",
    "crosses",
    "decide",
    "def",
    "dependency",
    "doc",
    "enum",
    "event",
    "flow",
    "from",
    "import",
    "in",
    "inout",
    "interface",
    "item",
    "library",
    "package",
    "part",
    "perform",
    "port",
    "private",
    "protected",
    "public",
    "references",
    "ref",
    "redefines",
    "requirement",
    "satisfy",
    "specializes",
    "state",
    "subsets",
    "to",
    "use",
    "view",
    "viewpoint",
}

TWO_CHAR_OPERATORS = {":>", "::", ":=", ":>>", "::>", "=>", "->", ".."}
SYMBOLS = set("{}()[];,=<>.*~")


class Scanner:
    def __init__(self, path: Path, text: str) -> None:
        self.path = path
        self.text = text
        self.index = 0
        self.line = 1
        self.column = 1
        self.diagnostics: list[Diagnostic] = []

    def scan(self) -> tuple[list[Token], list[Diagnostic]]:
        tokens: list[Token] = []
        while not self._at_end():
            char = self._peek()
            if char in " \t\r":
                self._advance()
            elif char == "\n":
                self._advance_line()
            elif char == "/" and self._peek_next() == "/":
                self._scan_line_comment()
            elif char == "/" and self._peek_next() == "*":
                self._scan_block_comment()
            elif char in {'"', "'"}:
                tokens.append(self._scan_quoted())
            elif char.isalpha() or char == "_":
                tokens.append(self._scan_identifier())
            elif char.isdigit():
                tokens.append(self._scan_number())
            elif ord(char) < 32:
                self._diagnose("SYSML001", "Invalid control character in source text.", self.line, self.column)
                self._advance()
            else:
                tokens.append(self._scan_symbol_or_operator())
        return tokens, self.diagnostics

    def _scan_identifier(self) -> Token:
        line, column = self.line, self.column
        value = ""
        while not self._at_end() and (self._peek().isalnum() or self._peek() in "_-"):
            value += self._advance()
        kind = TokenKind.KEYWORD if value in KEYWORDS else TokenKind.IDENTIFIER
        return Token(kind, value, line, column)

    def _scan_number(self) -> Token:
        line, column = self.line, self.column
        value = ""
        while not self._at_end() and (self._peek().isdigit() or self._peek() == "."):
            value += self._advance()
        return Token(TokenKind.NUMBER, value, line, column)

    def _scan_quoted(self) -> Token:
        quote = self._peek()
        line, column = self.line, self.column
        value = self._advance()
        escaped = False
        while not self._at_end():
            char = self._advance()
            value += char
            if char == "\n":
                self.line += 1
                self.column = 1
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                return Token(TokenKind.STRING, value, line, column)
        self._diagnose("SYSML002", "Unterminated string literal.", line, column)
        return Token(TokenKind.STRING, value, line, column)

    def _scan_line_comment(self) -> None:
        while not self._at_end() and self._peek() != "\n":
            self._advance()

    def _scan_block_comment(self) -> None:
        line, column = self.line, self.column
        self._advance()
        self._advance()
        while not self._at_end():
            if self._peek() == "*" and self._peek_next() == "/":
                self._advance()
                self._advance()
                return
            if self._peek() == "\n":
                self._advance_line()
            else:
                self._advance()
        self._diagnose("SYSML003", "Unterminated block comment.", line, column)

    def _scan_symbol_or_operator(self) -> Token:
        line, column = self.line, self.column
        three = self.text[self.index : self.index + 3]
        if three in TWO_CHAR_OPERATORS:
            self.index += 3
            self.column += 3
            return Token(TokenKind.OPERATOR, three, line, column)
        two = self.text[self.index : self.index + 2]
        if two in TWO_CHAR_OPERATORS:
            self.index += 2
            self.column += 2
            return Token(TokenKind.OPERATOR, two, line, column)
        value = self._advance()
        kind = TokenKind.SYMBOL if value in SYMBOLS else TokenKind.OPERATOR
        return Token(kind, value, line, column)

    def _diagnose(self, code: str, message: str, line: int, column: int) -> None:
        self.diagnostics.append(
            Diagnostic(Severity.ERROR, code, message, self.path, Position(line, column))
        )

    def _peek(self) -> str:
        return self.text[self.index]

    def _peek_next(self) -> str:
        if self.index + 1 >= len(self.text):
            return "\0"
        return self.text[self.index + 1]

    def _advance(self) -> str:
        char = self.text[self.index]
        self.index += 1
        self.column += 1
        return char

    def _advance_line(self) -> None:
        self.index += 1
        self.line += 1
        self.column = 1

    def _at_end(self) -> bool:
        return self.index >= len(self.text)
