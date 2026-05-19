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

> **Status.** v0.10.0 integrates the **tree-sitter SysML grammar**
> (US-201 Batch K). AST-aware declared-name collection augments the
> token-based recognizer in both `validate_reference_candidates` and
> the project-wide symbol index, cutting false-positive findings on
> the OMG corpus by 12% (examples) and 19% (validation). Token-based
> validators continue to run as the floor; the AST is additive while
> migration proceeds in Batch L.
>
> v0.9.0 shipped **Sysand project manifest discovery**
> (US-204) and a **differential-corpus regression harness** (US-207)
> that runs against the OMG `examples` (95 files) and `validation`
> (56 files) corpora and fails on count drift. See
> [`docs/differential-corpus-report.md`](docs/differential-corpus-report.md)
> for methodology, current state, and per-rule false-positive analysis.
> Side-by-side comparison against the OMG Pilot Implementation itself
> awaits Batch K (US-201 real parser).
>
> v0.7.0 shipped **structural rules over specialization /
> redefinition** (US-205, scoped): `SYSML210`/`SYSML211` flag
> unresolved `:>` and `:>>` targets as errors (not warnings),
> `SYSML212`/`SYSML213` flag self-reference, and `SYSML220` detects
> specialization cycles across the entire project's file set. All five
> rules run unconditionally — they're errors, not heuristic warnings.
> Verified against the [`scamp`](../scamp/) reference model (13 files,
> 6,015 LOC, 295 declarations) which passes clean.
>
> v0.6.x ships **cross-file name resolution** (US-203):
> `import` declarations are parsed per the normative KerML §8.2.3.4.2
> grammar (membership / namespace / recursive forms, visibility
> prefixes, `import all`), and a project-wide symbol table aggregates
> declarations across every file in the validation run. A `:> Engine`
> reference now resolves through any of: (1) the file's own
> declarations, (2) explicit and wildcard imports, (3) other files in
> the run, or (4) the embedded OMG standard library (v0.5.0).
>
> v0.5.0 ships the **embedded SysML v2 standard library** (OMG release
> `2026-04`, EPL-2.0, vendored at [`vendor/sysml-v2-release/`](vendor/)
> as a git submodule) indexing 94 files and ~4,190 declared symbols.
> See `sysml-validate library-info`.
>
> The Phase 1 government-acceptance envelope is intact (v0.4.0):
> SARIF / JUnit / JSON / text / plain output, SBOM + Sigstore + GPG +
> SLSA L3 via the [release workflow](.github/workflows/release.yml),
> reproducible builds, pinned toolchain, security / offline / threat
> model docs. Phase 2 continues with cross-file name resolution
> (US-203) and ported Pilot rules (US-205); see
> [`docs/PRD-government-readiness.md`](docs/PRD-government-readiness.md).

## Install

```powershell
git clone --recurse-submodules https://github.com/<owner>/sysml-cli
cd sysml-cli
cargo build --release
```

If you cloned without `--recurse-submodules`, run
`git submodule update --init --recursive` once before building — the
build embeds the vendored SysML v2 standard library at
[`vendor/sysml-v2-release/sysml.library/`](vendor/) into the binary.

The binary is in `target/release/sysml-validate(.exe)`. Runtime
dependencies: `sha2` (diagnostic fingerprints), `wait-timeout`
(official-backend timeout enforcement), `serde` + `serde_json` (SARIF
and baseline loading), `toml` (configuration file), `include_dir`
(embedded library). The glob matcher and JUnit XML emitter are
hand-written and dependency-free.

## Usage

Validate files or directories:

```powershell
sysml-validate validate .\model.sysml
sysml-validate validate .\sysml .\kerml --format json
sysml-validate validate .\sysml --ci             # shortcut for --format sarif
sysml-validate validate .\sysml --format junit   # Jenkins / GitLab pipelines
sysml-validate validate .\sysml --format plain   # screen-reader / grep-friendly
```

`--format plain` produces GCC-style one-diagnostic-per-line output
(`path:line:column: severity: code: message`) with no header and no
decoration. See [`docs/accessibility.md`](docs/accessibility.md) for the
Section 508 / VPAT 2.5 conformance statement.

`--format sarif` emits SARIF 2.1.0 — the lingua franca for GitHub Advanced
Security, GitLab Ultimate, Iron Bank, SonarQube, and Azure DevOps. Every
SARIF result carries a `partialFingerprints.diagnosticHash/v1` value
suitable for baseline diff and deduplication, plus a `suppressions[]`
marker (`kind: "inSource"`) for any diagnostic silenced by an inline
directive.

`--format junit` emits Maven Surefire-compatible XML. Each diagnostic
becomes a `<testcase>`; errors render as `<failure>`, warnings as
`<system-out>` (or `<error>` under `--fail-on-warning`).

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

## Configuration

`sysml-validate.toml` in the working directory or any ancestor is loaded
automatically. CLI flags always override config values. Use `--config <path>`
for an explicit file or `--no-config` to skip discovery entirely.

```toml
# sysml-validate.toml
project_root = "."
default_format = "sarif"
default_strict = true
default_fail_on_warning = false

# Glob patterns. Supports *, **, and ?. Path separators are normalized
# across Windows and POSIX. Include is an allowlist; exclude is applied
# after include.
include = ["src/**/*.sysml", "src/**/*.kerml"]
exclude = ["**/target/**", "**/generated/**"]

[rules]
SYSML040 = "error"   # promote unresolved-reference warning to error
SYSML041 = "off"     # suppress duplicate-member-in-scope entirely
```

The loaded config path appears in the JSON and SARIF metadata block, plus
the text header.

## Baseline / diff mode

Adopt the validator on a large existing project without fixing every
existing finding up front. Record a SARIF baseline of the current state,
then in CI only new findings fail the build:

```powershell
# Record the current findings as the accepted baseline.
sysml-validate validate .\src --ci --baseline .\baseline.sarif --update-baseline

# Subsequent CI runs: unchanged findings have baselineState=unchanged and
# don't affect the exit code; new findings do.
sysml-validate validate .\src --ci --baseline .\baseline.sarif
```

Both runs emit SARIF with `baselineState` on every result. Findings are
matched by `(ruleId, diagnosticHash/v1)`, so renaming an unrelated symbol
does not invalidate the baseline.

## Inline suppressions

Suppress diagnostics with a line comment:

```sysml
package P {
  // sysml-validate: disable=SYSML041
  part def Engine; part def Engine;       // same-line suppression

  // sysml-validate: disable-next-line=SYSML041,SYSML040
  part wheel :> Missing;

  // sysml-validate: disable=all
  alias E Engine;                          // every rule on this line
}
```

Suppression directives that don't match any diagnostic produce a `SYSML050`
warning so dead directives can be cleaned up. Invalid directive syntax
produces `SYSML060`.

Suppressed diagnostics are **kept in the diagnostic list** and marked with
`suppression`. They appear in SARIF as `results[].suppressions[]` entries
with `kind: "inSource"` (the SARIF-mandated audit record), and are hidden
from text and JSON output by default. Pass `--show-suppressed` to display
them in text/JSON. Suppressed diagnostics never affect the exit code.

Show the release conformance references, model corpora, and embedded
standard library:

```powershell
sysml-validate grammar-info
sysml-validate corpus-info
sysml-validate library-info             # 94 files, ~4,190 declared symbols
sysml-validate library-info --format json
```

Override the embedded library with an on-disk copy (e.g., to test
against a pre-release OMG library):

```powershell
sysml-validate validate .\model.sysml --strict `
  --library-path C:\path\to\sysml.library
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
| `SYSML040` | warning | Identifier reference not resolvable via any of: declared-in-file, project imports (membership / namespace / recursive), other files in the validation run, or the embedded SysML v2 standard library (with `--strict`). Checks `:` typed-usage, `:>` / `specializes` / `subsets`, `references` / `redefines`, and `for` / `to` / `from`. |
| `SYSML041` | error   | Duplicate member name in lexical scope. |
| `SYSML210` | error   | `:>` / `specializes` target does not resolve. |
| `SYSML211` | error   | `:>>` / `redefines` target does not resolve. |
| `SYSML212` | error   | Feature specializes itself. |
| `SYSML213` | error   | Feature redefines itself. |
| `SYSML220` | error   | Specialization graph contains a cycle (project-wide). |
| `SYSML050` | warning | Suppression directive did not match any diagnostic. |
| `SYSML060` | warning | Suppression directive has invalid syntax. |
| `SYSML800` | error   | Configuration file is invalid. |
| `SYSML900` | error   | `--official-command` parse/setup error. |
| `SYSML901` | error   | Official validator could not be executed. |
| `SYSML902` | error   | Official validator returned a non-zero exit status. |
| `SYSML903` | info    | Official validator returned informational output. |
| `SYSML904` | error   | Official validator exceeded `--timeout` and was terminated. |

When any rule's meaning changes, the rule catalog version in
[`src/report.rs`](src/report.rs) is bumped so consumers can gate baselines.
Current rule catalog version: **0.3.0**.

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
- No telemetry.
- `--official-command` is parsed into argv and invoked without a shell.
  Shell metacharacters in the template survive only as literal argv
  content; they are not interpreted.

These properties are intended for air-gapped and DoD IL4/IL5 deployment
contexts. Authoritative statements:

- [`docs/SECURITY.md`](docs/SECURITY.md) — vulnerability disclosure
  policy, release verification recipes (Sigstore + GPG + SLSA), NIST
  800-53 cross-references.
- [`docs/OFFLINE.md`](docs/OFFLINE.md) — per-subcommand network and
  filesystem surface, vendoring for air-gap builds.
- [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) — trust boundaries
  defended and explicitly out of scope.
- [`docs/REPRODUCING.md`](docs/REPRODUCING.md) — byte-identical rebuild
  recipe for independent verification.
- [`docs/accessibility.md`](docs/accessibility.md) — Section 508
  conformance, draft VPAT 2.5 (Rev 508), `--format plain` contract.

## License

MIT. See `LICENSE`.
