from __future__ import annotations

import subprocess
from pathlib import Path

from .models import Diagnostic, Position, Severity, ValidationResult
from .validator import NativeValidator


class BackendError(RuntimeError):
    pass


class ValidationBackend:
    def validate(self, path: Path) -> ValidationResult:
        raise NotImplementedError


class NativeBackend(ValidationBackend):
    def __init__(self, strict: bool = False) -> None:
        self.validator = NativeValidator(strict=strict)

    def validate(self, path: Path) -> ValidationResult:
        return self.validator.validate_path(path)


class OfficialCommandBackend(ValidationBackend):
    def __init__(self, command_template: str, timeout_seconds: int = 60) -> None:
        if "{file}" not in command_template:
            raise BackendError("--official-command must include a {file} placeholder.")
        self.command_template = command_template
        self.timeout_seconds = timeout_seconds

    def validate(self, path: Path) -> ValidationResult:
        command = self.command_template.format(file=str(path))
        result = ValidationResult(path)
        try:
            completed = subprocess.run(
                command,
                shell=True,
                check=False,
                capture_output=True,
                text=True,
                timeout=self.timeout_seconds,
            )
        except subprocess.TimeoutExpired:
            result.diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "SYSML900",
                    f"Official validator timed out after {self.timeout_seconds} seconds.",
                    path,
                )
            )
            return result
        except OSError as exc:
            result.diagnostics.append(
                Diagnostic(Severity.ERROR, "SYSML901", f"Unable to execute official validator: {exc}", path)
            )
            return result

        output = "\n".join(part for part in (completed.stdout.strip(), completed.stderr.strip()) if part)
        if completed.returncode != 0:
            result.diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "SYSML902",
                    output or f"Official validator exited with code {completed.returncode}.",
                    path,
                    Position(1, 1),
                )
            )
        elif output:
            result.diagnostics.append(
                Diagnostic(Severity.INFO, "SYSML903", output, path, Position(1, 1))
            )
        return result
