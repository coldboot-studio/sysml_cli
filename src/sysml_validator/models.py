from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any


class Severity(str, Enum):
    ERROR = "error"
    WARNING = "warning"
    INFO = "info"


@dataclass(frozen=True)
class Position:
    line: int
    column: int

    def to_json(self) -> dict[str, int]:
        return {"line": self.line, "column": self.column}


@dataclass(frozen=True)
class Diagnostic:
    severity: Severity
    code: str
    message: str
    path: Path
    position: Position | None = None

    def to_json(self) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "severity": self.severity.value,
            "code": self.code,
            "message": self.message,
            "path": str(self.path),
        }
        if self.position is not None:
            payload["position"] = self.position.to_json()
        return payload


@dataclass
class ValidationResult:
    path: Path
    diagnostics: list[Diagnostic] = field(default_factory=list)

    @property
    def error_count(self) -> int:
        return sum(1 for diagnostic in self.diagnostics if diagnostic.severity == Severity.ERROR)

    @property
    def warning_count(self) -> int:
        return sum(1 for diagnostic in self.diagnostics if diagnostic.severity == Severity.WARNING)

    @property
    def ok(self) -> bool:
        return self.error_count == 0

    def to_json(self) -> dict[str, Any]:
        return {
            "path": str(self.path),
            "ok": self.ok,
            "error_count": self.error_count,
            "warning_count": self.warning_count,
            "diagnostics": [diagnostic.to_json() for diagnostic in self.diagnostics],
        }
