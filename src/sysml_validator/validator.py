from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from .models import Diagnostic, Position, Severity, ValidationResult
from .release import SUPPORTED_EXTENSIONS
from .scanner import Scanner, Token, TokenKind


DEFINITION_KEYWORDS = {
    "attribute",
    "allocation",
    "analysis",
    "action",
    "calc",
    "case",
    "concern",
    "connection",
    "constraint",
    "enum",
    "flow",
    "interface",
    "item",
    "occurrence",
    "part",
    "port",
    "rendering",
    "requirement",
    "state",
    "use",
    "verification",
    "view",
    "viewpoint",
}

USAGE_KEYWORDS = {
    "action",
    "allocation",
    "assert",
    "attribute",
    "bind",
    "case",
    "concern",
    "connect",
    "connection",
    "constraint",
    "flow",
    "include",
    "interface",
    "item",
    "occurrence",
    "part",
    "perform",
    "port",
    "ref",
    "requirement",
    "satisfy",
    "state",
    "use",
    "verify",
    "view",
}

OPEN_TO_CLOSE = {"{": "}", "(": ")", "[": "]"}
CLOSE_TO_OPEN = {value: key for key, value in OPEN_TO_CLOSE.items()}
STATEMENT_END = {";", "{"}


@dataclass
class Scope:
    names: set[str]
    opened_by: Token


class NativeValidator:
    def __init__(self, strict: bool = False) -> None:
        self.strict = strict

    def validate_path(self, path: Path) -> ValidationResult:
        result = ValidationResult(path)
        if path.suffix.lower() not in SUPPORTED_EXTENSIONS:
            result.diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "SYSML010",
                    f"Unsupported file extension '{path.suffix}'. Expected .sysml or .kerml.",
                    path,
                )
            )
            return result
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            result.diagnostics.append(
                Diagnostic(Severity.ERROR, "SYSML011", "File is not valid UTF-8 text.", path)
            )
            return result
        except OSError as exc:
            result.diagnostics.append(
                Diagnostic(Severity.ERROR, "SYSML012", f"Unable to read file: {exc}", path)
            )
            return result

        scanner = Scanner(path, text)
        tokens, scan_diagnostics = scanner.scan()
        result.diagnostics.extend(scan_diagnostics)
        result.diagnostics.extend(self._validate_tokens(path, tokens))
        return result

    def _validate_tokens(self, path: Path, tokens: list[Token]) -> list[Diagnostic]:
        diagnostics: list[Diagnostic] = []
        diagnostics.extend(self._validate_balanced_delimiters(path, tokens))
        diagnostics.extend(self._validate_statement_shapes(path, tokens))
        diagnostics.extend(self._validate_duplicate_scope_members(path, tokens))
        if self.strict:
            diagnostics.extend(self._validate_reference_candidates(path, tokens))
        return diagnostics

    def _validate_balanced_delimiters(self, path: Path, tokens: list[Token]) -> list[Diagnostic]:
        diagnostics: list[Diagnostic] = []
        stack: list[Token] = []
        for token in tokens:
            if token.value in OPEN_TO_CLOSE:
                stack.append(token)
            elif token.value in CLOSE_TO_OPEN:
                if not stack or stack[-1].value != CLOSE_TO_OPEN[token.value]:
                    diagnostics.append(
                        Diagnostic(
                            Severity.ERROR,
                            "SYSML020",
                            f"Unmatched closing delimiter '{token.value}'.",
                            path,
                            token.position,
                        )
                    )
                else:
                    stack.pop()
        for token in reversed(stack):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "SYSML021",
                    f"Unclosed delimiter '{token.value}', expected '{OPEN_TO_CLOSE[token.value]}'.",
                    path,
                    token.position,
                )
            )
        return diagnostics

    def _validate_statement_shapes(self, path: Path, tokens: list[Token]) -> list[Diagnostic]:
        diagnostics: list[Diagnostic] = []
        for index, token in enumerate(tokens):
            if token.value == "package":
                diagnostics.extend(self._expect_name_and_terminator(path, tokens, index, "package"))
            elif token.value == "library":
                package_index = self._skip_metadata(tokens, index + 1)
                if package_index >= len(tokens) or tokens[package_index].value != "package":
                    diagnostics.append(
                        Diagnostic(
                            Severity.ERROR,
                            "SYSML030",
                            "Expected 'package' after 'library'.",
                            path,
                            token.position,
                        )
                    )
            elif token.value == "import":
                diagnostics.extend(self._expect_terminator(path, tokens, index, "import declaration"))
            elif token.value == "alias":
                diagnostics.extend(self._validate_alias(path, tokens, index))
            elif token.value == "dependency":
                diagnostics.extend(self._validate_dependency(path, tokens, index))
            elif token.value in DEFINITION_KEYWORDS and self._next_value(tokens, index) == "def":
                diagnostics.extend(self._expect_name_and_terminator(path, tokens, index + 1, f"{token.value} definition"))
            elif token.value in USAGE_KEYWORDS:
                diagnostics.extend(self._validate_usage_statement(path, tokens, index))
        return diagnostics

    def _validate_alias(self, path: Path, tokens: list[Token], index: int) -> list[Diagnostic]:
        diagnostics = self._expect_terminator(path, tokens, index, "alias declaration")
        if not self._contains_before_end(tokens, index, "for"):
            diagnostics.append(
                Diagnostic(Severity.ERROR, "SYSML031", "Alias declaration must include 'for'.", path, tokens[index].position)
            )
        return diagnostics

    def _validate_dependency(self, path: Path, tokens: list[Token], index: int) -> list[Diagnostic]:
        diagnostics = self._expect_terminator(path, tokens, index, "dependency")
        if not self._contains_before_end(tokens, index, "to"):
            diagnostics.append(
                Diagnostic(Severity.ERROR, "SYSML032", "Dependency declaration must include a supplier after 'to'.", path, tokens[index].position)
            )
        return diagnostics

    def _validate_usage_statement(self, path: Path, tokens: list[Token], index: int) -> list[Diagnostic]:
        if self._previous_value(tokens, index) == "def":
            return []
        if self._is_connector_short_form(tokens[index].value):
            return self._expect_terminator(path, tokens, index, f"{tokens[index].value} usage")
        next_token = tokens[index + 1] if index + 1 < len(tokens) else None
        if next_token is None or next_token.value in STATEMENT_END:
            return [
                Diagnostic(
                    Severity.ERROR,
                    "SYSML033",
                    f"Expected a declared name or specialization after '{tokens[index].value}'.",
                    path,
                    tokens[index].position,
                )
            ]
        return self._expect_terminator(path, tokens, index, f"{tokens[index].value} usage")

    def _expect_name_and_terminator(
        self, path: Path, tokens: list[Token], index: int, statement: str
    ) -> list[Diagnostic]:
        diagnostics: list[Diagnostic] = []
        name_index = index + 1
        if index < len(tokens) and tokens[index].value == "def":
            name_index = index + 1
        if name_index >= len(tokens) or tokens[name_index].value in STATEMENT_END:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "SYSML034",
                    f"Expected a name for {statement}.",
                    path,
                    tokens[index].position,
                )
            )
        diagnostics.extend(self._expect_terminator(path, tokens, index, statement))
        return diagnostics

    def _expect_terminator(
        self, path: Path, tokens: list[Token], index: int, statement: str
    ) -> list[Diagnostic]:
        depth = {"{": 0, "(": 0, "[": 0}
        cursor = index + 1
        while cursor < len(tokens):
            value = tokens[cursor].value
            if value in depth:
                if value == "{" and depth == {"{": 0, "(": 0, "[": 0}:
                    return []
                depth[value] += 1
            elif value in CLOSE_TO_OPEN:
                opener = CLOSE_TO_OPEN[value]
                if opener in depth and depth[opener] > 0:
                    depth[opener] -= 1
            elif value == ";" and all(count == 0 for count in depth.values()):
                return []
            cursor += 1
        return [
            Diagnostic(
                Severity.ERROR,
                "SYSML035",
                f"Expected ';' or '{{' to terminate {statement}.",
                path,
                tokens[index].position,
            )
        ]

    def _validate_duplicate_scope_members(self, path: Path, tokens: list[Token]) -> list[Diagnostic]:
        diagnostics: list[Diagnostic] = []
        scopes = [Scope(set(), Token(TokenKind.SYMBOL, "{", 1, 1))]
        pending_name: str | None = None
        pending_token: Token | None = None
        for index, token in enumerate(tokens):
            if token.value == "{":
                scopes.append(Scope(set(), token))
                if pending_name and len(scopes) >= 2:
                    self._record_name(path, scopes[-2], pending_name, pending_token or token, diagnostics)
                pending_name = None
                pending_token = None
            elif token.value == "}":
                if len(scopes) > 1:
                    scopes.pop()
            elif token.value == ";":
                if pending_name:
                    self._record_name(path, scopes[-1], pending_name, pending_token or token, diagnostics)
                pending_name = None
                pending_token = None
            elif self._starts_named_member(tokens, index):
                name = self._declared_name_after(tokens, index)
                if name is not None:
                    pending_name = name.value
                    pending_token = name
        return diagnostics

    def _validate_reference_candidates(self, path: Path, tokens: list[Token]) -> list[Diagnostic]:
        diagnostics: list[Diagnostic] = []
        declared = {
            token.value
            for index, token in enumerate(tokens)
            if token.kind == TokenKind.IDENTIFIER and self._is_declaration_name(tokens, index)
        }
        reference_markers = {"for", "to", "from", ":>", "specializes", "subsets", "references", "redefines"}
        for index, token in enumerate(tokens[:-1]):
            if token.value in reference_markers:
                candidate = tokens[index + 1]
                if candidate.kind == TokenKind.IDENTIFIER and candidate.value not in declared:
                    diagnostics.append(
                        Diagnostic(
                            Severity.WARNING,
                            "SYSML040",
                            f"Reference '{candidate.value}' is not declared in this file. It may resolve from imports or libraries.",
                            path,
                            candidate.position,
                        )
                    )
        return diagnostics

    def _record_name(
        self,
        path: Path,
        scope: Scope,
        name: str,
        token: Token,
        diagnostics: list[Diagnostic],
    ) -> None:
        if name in scope.names:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "SYSML041",
                    f"Duplicate member name '{name}' in the same lexical scope.",
                    path,
                    token.position,
                )
            )
        scope.names.add(name)

    def _starts_named_member(self, tokens: list[Token], index: int) -> bool:
        token = tokens[index]
        if token.value == "package":
            return True
        if token.value in DEFINITION_KEYWORDS and self._next_value(tokens, index) == "def":
            return True
        if token.value in USAGE_KEYWORDS and self._previous_value(tokens, index) != "def":
            return True
        return False

    def _declared_name_after(self, tokens: list[Token], index: int) -> Token | None:
        cursor = index + 1
        if cursor < len(tokens) and tokens[cursor].value == "def":
            cursor += 1
        while cursor < len(tokens):
            token = tokens[cursor]
            if token.kind == TokenKind.IDENTIFIER:
                return token
            if token.value in STATEMENT_END:
                return None
            cursor += 1
        return None

    def _is_declaration_name(self, tokens: list[Token], index: int) -> bool:
        for cursor in range(max(0, index - 3), index):
            if self._starts_named_member(tokens, cursor) and self._declared_name_after(tokens, cursor) == tokens[index]:
                return True
        return False

    def _contains_before_end(self, tokens: list[Token], index: int, value: str) -> bool:
        cursor = index + 1
        while cursor < len(tokens) and tokens[cursor].value not in {";", "{"}:
            if tokens[cursor].value == value:
                return True
            cursor += 1
        return False

    def _next_value(self, tokens: list[Token], index: int) -> str | None:
        if index + 1 >= len(tokens):
            return None
        return tokens[index + 1].value

    def _previous_value(self, tokens: list[Token], index: int) -> str | None:
        if index == 0:
            return None
        return tokens[index - 1].value

    def _skip_metadata(self, tokens: list[Token], index: int) -> int:
        return index

    def _is_connector_short_form(self, value: str) -> bool:
        return value in {"bind", "connect", "satisfy", "verify", "include", "perform", "assert"}
