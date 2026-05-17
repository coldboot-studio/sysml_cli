from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from sysml_validator.validator import NativeValidator


class NativeValidatorTests(unittest.TestCase):
    def validate_text(self, text: str, suffix: str = ".sysml", strict: bool = False):
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / f"model{suffix}"
            path.write_text(text, encoding="utf-8")
            return NativeValidator(strict=strict).validate_path(path)

    def test_accepts_basic_package(self) -> None:
        result = self.validate_text(
            """
            package Vehicle {
              part def Engine;
              part engine : Engine;
              attribute mass;
            }
            """
        )
        self.assertEqual([], [d.message for d in result.diagnostics if d.severity.value == "error"])

    def test_reports_unbalanced_delimiter(self) -> None:
        result = self.validate_text("package Vehicle { part def Engine;")
        self.assertFalse(result.ok)
        self.assertTrue(any(d.code == "SYSML021" for d in result.diagnostics))

    def test_reports_missing_alias_for(self) -> None:
        result = self.validate_text("package P { alias E Engine; }")
        self.assertFalse(result.ok)
        self.assertTrue(any(d.code == "SYSML031" for d in result.diagnostics))

    def test_reports_duplicate_member_in_scope(self) -> None:
        result = self.validate_text("package P { part def Engine; part def Engine; }")
        self.assertFalse(result.ok)
        self.assertTrue(any(d.code == "SYSML041" for d in result.diagnostics))

    def test_rejects_unsupported_extension(self) -> None:
        result = self.validate_text("package P;", suffix=".txt")
        self.assertFalse(result.ok)
        self.assertTrue(any(d.code == "SYSML010" for d in result.diagnostics))

    def test_strict_reference_warning(self) -> None:
        result = self.validate_text("package P { part engine :> Missing; }", strict=True)
        self.assertTrue(any(d.code == "SYSML040" for d in result.diagnostics))


if __name__ == "__main__":
    unittest.main()
