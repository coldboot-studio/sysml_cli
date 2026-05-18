# sysml-validate

`sysml-validate` is a Rust CLI **preflight** validator for SysML v2 and KerML
textual models. It is aligned with the public release layout from
[Systems-Modeling/SysML-v2-Release](https://github.com/Systems-Modeling/SysML-v2-Release):

- textual grammar references live in `bnf/`, including `SysML-textual-bnf.kebnf`
  and `KerML-textual-bnf.kebnf`
- example projects live in `sysml/` and `kerml/`
- normative model libraries live in `sysml.library/`

The native backend is intentionally conservative: it catches deterministic
textual issues without pretending to replace the full reference implementation.
For full conformance checking, use `--backend official` with a local command
that invokes the SysML v2 pilot/release tooling.

> **Status.** Preflight only today. See [`docs/PRD-government-readiness.md`](docs/PRD-government-readiness.md)
> for the roadmap to a real conformance-grade validator with US Government
> acceptance artifacts (SARIF, SBOM, SLSA L3 provenance, NIST SSDF mapping,
> optional DO-330 TQL-5 qualification kit).

## Install

```powershell
cargo build --release
```

The binary is in `target/release/sysml-validate(.exe)`. Runtime dependencies
are limited to `sha2` (diagnostic fingerprints) and `wait-timeout` (official-
backend timeout enforcement).

## Usage

Validate files or directories:

```powershell
sysml-validate validate .\model.sysml
sysml-validate validate .\sysml .\kerml --format json
```

Delegate to the official backend. `{file}` is replaced with each model path.
The argument list is shell-style tokenized (single quotes, double quotes,
`\\` escapes inside double quotes) and invoked with positional argv — **no
shell process is spawned**, so `--official-command` cannot inject shell
metacharacters. If the child exceeds `--timeout` seconds it is terminated
and a `SYSML904` diagnostic is emitted.

```powershell
sysml-validate validate .\model.sysml --backend official `
  --official-command "sysml-validator --strict {file}" `
  --timeout 120
```

Fail the run on warnings (useful in strict CI gates):

```powershell
sysml-validate validate .\model.sysml --strict --fail-on-warning
```

Show the release conformance references and model corpora the tool knows
about:

```powershell
sysml-validate grammar-info
sysml-validate corpus-info
```

## Output

JSON output includes a `metadata` block with the tool name and version, rule
catalog version, invocation timestamp (RFC 3339 UTC), backend identity, and
ruleset flags. Each diagnostic also includes a stable `fingerprint` — a
SHA-256-derived hash of `(rule code, normalized file path, genericized
message)` that is intentionally **position-independent**, so inserting an
unrelated line will not change the fingerprint of diagnostics that follow.
The metadata block plus per-diagnostic fingerprints are the per-run audit
record consumers should retain alongside their build provenance.

```json
{
  "metadata": {
    "tool": {"name": "sysml-validate", "version": "0.2.0"},
    "rule_catalog": {"version": "0.1.0"},
    "invocation": {
      "timestamp_utc": "2026-05-18T13:00:46Z",
      "timestamp_epoch_seconds": 1779109246,
      "backend": "native",
      "strict": false,
      "format": "json"
    }
  },
  "results": [ /* ... */ ]
}
```

## Public Test Models

`corpus-info` lists online SysML v2 model corpora that are useful for smoke
tests:

- `Systems-Modeling/SysML-v2-Release`: official examples, training, validation,
  and library models.
- `GfSE/SysML-v2-Models`: community-curated textual SysML v2 models.
- `sensmetry/advent-of-sysml-v2`: lesson-oriented SysML v2 examples.
- `sensmetry/smart-home-hub-example`: a small complete architecture example.
- OMG machine-readable SysML files, including the Simple Vehicle Model.

## Native Checks (today)

The built-in backend validates:

- supported file extensions: `.sysml` and `.kerml`
- balanced braces, parentheses, and brackets
- unterminated string/block/comment constructs
- invalid control characters
- declaration terminators for common SysML/KerML constructs
- basic package/import/alias/dependency/definition/usage statement shape
- duplicate package member names in the same lexical scope
- optional unresolved qualified-name references with `--strict`

Exit codes:

- `0`: no errors
- `1`: validation errors found
- `2`: CLI or backend configuration error

## Diagnostic Code Catalog

Rule codes use the namespace `SYSML0xx`. The current set:

| Code      | Severity | Meaning |
|-----------|----------|---------|
| `SYSML001` | error   | Invalid control character in source text. |
| `SYSML002` | error   | Unterminated string literal. |
| `SYSML003` | error   | Unterminated block comment. |
| `SYSML010` | error   | Unsupported file extension. |
| `SYSML012` | error   | Unable to read file as UTF-8 text. |
| `SYSML020` | error   | Unmatched closing delimiter. |
| `SYSML021` | error   | Unclosed delimiter. |
| `SYSML030` | error   | Expected `package` after `library`. |
| `SYSML031` | error   | Alias declaration must include `for`. |
| `SYSML032` | error   | Dependency must include a supplier after `to`. |
| `SYSML033` | error   | Usage missing a declared name or specialization. |
| `SYSML034` | error   | Definition missing a name. |
| `SYSML035` | error   | Missing `;` or `{` terminator. |
| `SYSML040` | warning | Identifier reference not declared in this file (with `--strict`). |
| `SYSML041` | error   | Duplicate member name in lexical scope. |
| `SYSML900` | error   | `--official-command` parse/setup error. |
| `SYSML901` | error   | Official validator could not be executed. |
| `SYSML902` | error   | Official validator returned a non-zero exit status. |
| `SYSML903` | info    | Official validator returned informational output. |
| `SYSML904` | error   | Official validator exceeded `--timeout` and was terminated. |

When any rule's meaning changes, the rule catalog version in
[`src/report.rs`](src/report.rs) is bumped so consumers can gate baselines.
Current rule catalog version: **0.2.0**.

## Scope

SysML v2 has a formal abstract syntax, semantic constraints, and model library
resolution rules. The native backend is useful in CI as a fast preflight gate,
but full language conformance must today be delegated to the official
release/pilot implementation backend. See
[`docs/PRD-government-readiness.md`](docs/PRD-government-readiness.md) for
the planned path to a real validator.

## Security Posture

- No network access in any subcommand.
- No automatic updates.
- `--official-command` is parsed into argv and invoked without a shell. Shell
  metacharacters in the template survive only as literal argv content; they
  are not interpreted.
- No telemetry.

These properties are intended for air-gapped and DoD IL4/IL5 deployment
contexts. They will be re-asserted in a `SECURITY.md` and `OFFLINE.md` in
Phase 1 of the roadmap.

## License

MIT. See `LICENSE`.
