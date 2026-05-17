from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Iterable

from .backends import BackendError, NativeBackend, OfficialCommandBackend, ValidationBackend
from .models import Severity, ValidationResult
from .release import GRAMMAR_REFERENCES, MODEL_PROJECTS, RELEASE_BRANCH, RELEASE_REPOSITORY, SUPPORTED_EXTENSIONS


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "validate":
        return run_validate(args)
    if args.command == "grammar-info":
        return run_grammar_info(args)
    parser.print_help()
    return 2


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="sysml-validate",
        description="Validate SysML v2 and KerML textual model files.",
    )
    subparsers = parser.add_subparsers(dest="command")

    validate = subparsers.add_parser("validate", help="Validate files or directories.")
    validate.add_argument("paths", nargs="+", type=Path, help="Files or directories to validate.")
    validate.add_argument("--format", choices=("text", "json"), default="text", help="Output format.")
    validate.add_argument("--strict", action="store_true", help="Enable warnings for references not declared in the same file.")
    validate.add_argument("--backend", choices=("native", "official"), default="native", help="Validation backend.")
    validate.add_argument("--official-command", help="Command template for official backend. Must include {file}.")
    validate.add_argument("--timeout", type=int, default=60, help="Official backend timeout per file in seconds.")

    grammar = subparsers.add_parser("grammar-info", help="Print SysML v2 release grammar references.")
    grammar.add_argument("--format", choices=("text", "json"), default="text", help="Output format.")
    return parser


def run_validate(args: argparse.Namespace) -> int:
    try:
        backend = make_backend(args)
    except BackendError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    files = list(discover_files(args.paths))
    if not files:
        print("error: no .sysml or .kerml files found", file=sys.stderr)
        return 2

    results = [backend.validate(path) for path in files]
    if args.format == "json":
        print(json.dumps({"results": [result.to_json() for result in results]}, indent=2))
    else:
        print_text_results(results)
    return 1 if any(not result.ok for result in results) else 0


def run_grammar_info(args: argparse.Namespace) -> int:
    payload = {
        "repository": RELEASE_REPOSITORY,
        "branch": RELEASE_BRANCH,
        "model_projects": list(MODEL_PROJECTS),
        "grammars": GRAMMAR_REFERENCES,
    }
    if args.format == "json":
        print(json.dumps(payload, indent=2))
    else:
        print(f"Repository: {RELEASE_REPOSITORY}")
        print(f"Branch: {RELEASE_BRANCH}")
        print("Model projects: " + ", ".join(MODEL_PROJECTS))
        print("Grammar references:")
        for name, reference in GRAMMAR_REFERENCES.items():
            print(f"  {name}: {reference['path']}")
            print(f"    {reference['url']}")
    return 0


def make_backend(args: argparse.Namespace) -> ValidationBackend:
    if args.backend == "native":
        return NativeBackend(strict=args.strict)
    if not args.official_command:
        raise BackendError("--backend official requires --official-command.")
    return OfficialCommandBackend(args.official_command, timeout_seconds=args.timeout)


def discover_files(paths: Iterable[Path]) -> Iterable[Path]:
    for path in paths:
        resolved = path.resolve()
        if resolved.is_file() and resolved.suffix.lower() in SUPPORTED_EXTENSIONS:
            yield resolved
        elif resolved.is_dir():
            for extension in sorted(SUPPORTED_EXTENSIONS):
                yield from sorted(resolved.rglob(f"*{extension}"))


def print_text_results(results: list[ValidationResult]) -> None:
    for result in results:
        status = "OK" if result.ok else "FAIL"
        print(f"{status} {result.path}")
        for diagnostic in result.diagnostics:
            location = ""
            if diagnostic.position is not None:
                location = f":{diagnostic.position.line}:{diagnostic.position.column}"
            prefix = diagnostic.severity.value.upper()
            print(f"  {prefix} {diagnostic.code}{location} {diagnostic.message}")

    total_errors = sum(result.error_count for result in results)
    total_warnings = sum(result.warning_count for result in results)
    print(f"\nValidated {len(results)} file(s): {total_errors} error(s), {total_warnings} warning(s).")


if __name__ == "__main__":
    raise SystemExit(main())
