# sysml-validate Technical Manual

| Field | Value |
|---|---|
| Product | `sysml-validate` |
| Version | 0.15.0 |
| Document revision | 1.0 |
| Last updated | 2026-05-19 |
| Audience | End users — model authors, CI engineers, release managers, government operators |
| Scope | Day-to-day operation of `sysml-validate`. Companion to [`EXECUTIVE_SUMMARY.md`](EXECUTIVE_SUMMARY.md), [`PRD-government-readiness.md`](PRD-government-readiness.md), and the compliance bundle at [`compliance/INDEX.md`](compliance/INDEX.md). |

---

## Contents

1. [Introduction & Scope](#1-introduction--scope)
2. [System Description](#2-system-description)
3. [Installation](#3-installation)
4. [Operation — CLI Reference](#4-operation--cli-reference)
5. [Configuration Reference](#5-configuration-reference)
6. [Output Formats](#6-output-formats)
7. [Rule Catalog](#7-rule-catalog)
8. [Suppressions](#8-suppressions)
9. [Baseline / Diff Workflow](#9-baseline--diff-workflow)
10. [LSP Integration](#10-lsp-integration)
11. [CI/CD Integration Patterns](#11-cicd-integration-patterns)
12. [Troubleshooting](#12-troubleshooting)
13. [Security & Trust Verification](#13-security--trust-verification)
14. [Compliance Reference](#14-compliance-reference)
15. [Glossary](#15-glossary)

Appendices:
- [A. Sample configurations](#appendix-a-sample-configurations)
- [B. Air-gap deployment recipe](#appendix-b-air-gap-deployment-recipe)
- [C. Document conventions](#appendix-c-document-conventions)

---

## 1. Introduction & Scope

### 1.1 Purpose

`sysml-validate` is a command-line preflight validator for **SysML v2**
and **KerML** textual models. It is designed to run as a CI gate
between commit and review, catching structural / reference / hygiene
errors quickly and emitting findings in the formats your existing
DevSecOps stack already consumes (SARIF, JUnit, JSON, plain text).

### 1.2 What this manual covers

This manual is the **operator's reference**. It documents every CLI
flag, every configuration knob, every output format, every rule
emitted, every suppression mechanism, and the recipes for integrating
the tool with major CI systems and editors. It is intended for users
running the tool, not for developers modifying the tool's source.

For development / contribution information, see the project README and
the PRD ([`PRD-government-readiness.md`](PRD-government-readiness.md)).
For trust-verification recipes (signatures, SBOM, SLSA), see
[`SECURITY.md`](SECURITY.md). For accessibility statements, see
[`accessibility.md`](accessibility.md).

**For AI-augmented users.** If you are using an agentic coding tool
(Claude Code, Cursor, Gemini CLI, OpenCode, Junie, OpenHands, GitHub
Copilot, Goose, Amp, Roo Code, etc.), this project ships an
[agentskills.io](https://agentskills.io)-spec agent skill at
[`skills/sysml-validate/SKILL.md`](../skills/sysml-validate/SKILL.md).
Drop the `skills/sysml-validate/` directory into your agent's skills
path and your agent gains imperative instructions for invoking the
validator, the full `SYSMLxxx` remediation table, the exact
suppression-directive grammar, and per-platform CI recipes — loaded
on demand via progressive disclosure.

### 1.3 What this manual does NOT cover

- The OMG SysML v2 / KerML languages themselves. See the OMG
  specifications listed in [`PRD-government-readiness.md`](PRD-government-readiness.md) §9.
- Internals of the validator (parser, AST walkers, project index).
  See the source under `src/`.
- The OMG Pilot Implementation, which `sysml-validate` can delegate to
  via `--backend official` but does not include or replace.

### 1.4 Reference documents

| Tag | Document |
|---|---|
| [SEC] | [`SECURITY.md`](SECURITY.md) — vulnerability disclosure, signature verification |
| [OFF] | [`OFFLINE.md`](OFFLINE.md) — air-gap deployment, network surface |
| [TM] | [`THREAT_MODEL.md`](THREAT_MODEL.md) — trust boundaries |
| [REP] | [`REPRODUCING.md`](REPRODUCING.md) — byte-identical rebuild |
| [ACC] | [`accessibility.md`](accessibility.md) — Section 508 / VPAT |
| [PRD] | [`PRD-government-readiness.md`](PRD-government-readiness.md) — full requirement set |
| [CI] | [`compliance/INDEX.md`](compliance/INDEX.md) — conformance bundle index |

---

## 2. System Description

### 2.1 What sysml-validate does

`sysml-validate` reads `.sysml` and `.kerml` files (or directories
containing them) and emits diagnostics. The native backend performs:

- Lexical correctness (encoding, control characters, unterminated
  strings / comments).
- Balanced delimiter checking (`{`, `(`, `[`).
- Statement-shape recognition for SysML / KerML constructs.
- Duplicate-member-in-scope detection.
- Cross-file qualified-name resolution under `--strict`.
- Specialization / redefinition target resolution and self-reference
  detection.
- Specialization-cycle detection across the project.
- Project-manifest discovery (Sysand `.project.json`).
- AST-aware tree-sitter parser, used additively to close the largest
  false-positive classes from the token-only walker.
- Optional delegation to a configured "official" backend via
  `--backend official --official-command "..."`.

### 2.2 What sysml-validate does NOT do

- It is **not** a full implementation of the OMG SysML v2 / KerML
  specification. Deep semantic checks (constraint evaluation,
  expression evaluation, full type conformance) require the OMG Pilot
  Implementation. Use `--backend official` to delegate to it.
- It does **not** transform models (no v1→v2 migration, no rendering,
  no transformation to other formats besides diagnostic outputs).
- It does **not** access the network. See [OFF].
- It does **not** auto-update or emit telemetry. See [OFF].

### 2.3 Backends

| Backend | Selected by | Behavior |
|---|---|---|
| `native` | default | Built-in validator. Fast, no external dependencies, the recommended default for CI gates. |
| `official` | `--backend official` + `--official-command "<argv template>"` | Delegates each file to a user-supplied command (typically the OMG Pilot). Tokenized with shell-style quoting, invoked with positional argv — **no shell process is spawned**. Killed if it exceeds `--timeout` (default 60s) with a `SYSML904` diagnostic. |

### 2.4 Security posture summary

- **No network access** in any subcommand.
- **No telemetry**, **no auto-update**.
- `--official-command` is argv-invoked (no shell metacharacter
  interpretation).
- Signed, reproducible, SBOM-published, SLSA-attested releases.

Full statements: [SEC], [OFF], [TM], [REP].

---

## 3. Installation

### 3.1 Prerequisites

- A 64-bit OS. Supported targets: `x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`,
  `x86_64-apple-darwin`, `aarch64-apple-darwin`.
- For source builds: Rust 1.85.0 or later (pinned in
  [`rust-toolchain.toml`](../rust-toolchain.toml)), Git with submodule
  support.

### 3.1.1 What ships (and what doesn't)

The deliverable is **one archive per target triple**, attached to each
GitHub Release tag:

- `sysml-validate-<version>-<target>.tar.gz` on Linux / macOS
- `sysml-validate-<version>-<target>.zip` on Windows

Each archive is ~1-5 MB compressed and expands to the structured tree
documented in [`compliance/INDEX.md`](compliance/INDEX.md): the
binary, all signatures and provenance, both SBOMs, every doc in this
bundle, the agent skill, LICENSE / NOTICE / VERSION, and a
`BUNDLE-MANIFEST.txt` listing the SHA-256 of every file.

The source-tree directories `target/`, `vendor/`, `tests/`, `src/`,
and `.github/` are **not** part of the deliverable. `target/` is
Cargo's build directory (working state); `vendor/` is a pinned
submodule of the OMG SysML v2 standard library that is embedded
into the binary at compile time and not needed at runtime.

Maintainers can produce a local copy of the deliverable (minus the
cryptographic trust artifacts that require GitHub OIDC) with:

```sh
scripts/build-local-bundle.sh
# Output: dist/local/bundle/sysml-validate-<version>-<target>.tar.gz
```

This is the right way to inspect the layout an end user sees before
cutting a real release.

### 3.2 From signed release (recommended)

1. Download the appropriate archive for your target from the GitHub
   Releases page:
   ```
   sysml-validate-<version>-<target>.tar.gz   # Linux, macOS
   sysml-validate-<version>-<target>.zip      # Windows
   ```
2. Verify the archive's signatures before extracting. See §13.
3. Extract:
   ```sh
   tar -xzf sysml-validate-0.15.0-x86_64-unknown-linux-gnu.tar.gz
   cd sysml-validate-0.15.0-x86_64-unknown-linux-gnu
   ```
4. The binary is at `bin/sysml-validate` (or `bin\sysml-validate.exe`
   on Windows). Place it on `PATH`, or run it from its current
   location.

### 3.3 From source

```sh
git clone --recurse-submodules https://github.com/<owner>/sysml-cli
cd sysml-cli
cargo build --release --locked
```

The compiled binary is at `target/release/sysml-validate(.exe)`. The
build embeds the vendored SysML v2 standard library at
`vendor/sysml-v2-release/sysml.library/` into the binary — that
submodule MUST be present at build time (the `--recurse-submodules`
clone or `git submodule update --init --recursive` ensures this).

### 3.4 Air-gap installation

See [OFF] for the canonical recipe. Summary:

1. On an internet-connected host, run `cargo vendor` to capture
   crate sources into `vendor/`. Commit and ship the resulting
   `vendor/` tree alongside the source archive.
2. On the air-gapped host, build with the Cargo `[source]` override
   documented in [OFF] so the build uses the vendored sources.
3. Or: deploy the pre-built signed binary directly. The binary has no
   runtime dependencies and includes the standard library, so it
   needs no install-time network access.

### 3.5 Verifying the install

```sh
sysml-validate --version
# sysml-validate 0.15.0

sysml-validate library-info
# Source: embedded SysML v2 standard library (OMG release 2026-04)
# Files:  94
# Declarations: 4190
# Packages: ...
```

A successful `library-info` confirms the embedded standard library
loaded — a strong signal the binary is intact.

---

## 4. Operation — CLI Reference

### 4.1 Synopsis

```
sysml-validate <SUBCOMMAND> [OPTIONS]
sysml-validate --help | --version
```

Subcommands: `validate`, `grammar-info`, `corpus-info`,
`library-info`, `lsp`.

Append `--help` to any subcommand for that subcommand's full option
list with examples.

### 4.2 sysml-validate validate

Validate one or more `.sysml` / `.kerml` files or directories.
Directories are walked recursively.

**Options:**

| Flag | Description |
|---|---|
| `--format <FORMAT>` | `text` (default), `plain`, `json`, `sarif`, or `junit`. See §6. |
| `--ci` | Shortcut for `--format sarif`. |
| `--strict` | Warn on unresolved identifier references (SYSML040). |
| `--fail-on-warning` | Exit 1 if any warning is produced. |
| `--show-suppressed` | Include suppressed diagnostics in text/JSON output. (SARIF always includes them as `suppressions[]`.) |
| `--config <PATH>` | Use an explicit `sysml-validate.toml`. |
| `--no-config` | Skip config-file discovery entirely. |
| `--library-path <DIR>` | Override the embedded standard library with an on-disk copy. |
| `--baseline <PATH>` | Classify findings against a prior SARIF run. See §9. |
| `--update-baseline` | Overwrite `--baseline` with the current run. Requires `--format sarif` and `--baseline`. |
| `--backend <native\|official>` | Validator backend. Default `native`. |
| `--official-command <TPL>` | Argv template for `--backend official`. `{file}` is replaced with each model path. |
| `--timeout <SECONDS>` | Kill the official backend if it exceeds this. Default 60. |
| `-h, --help` | Show full help with examples. |

**Exit codes:**

| Code | Meaning |
|---|---|
| 0 | No errors found (or `--update-baseline` accepted the current state). |
| 1 | Validation errors found (or warnings under `--fail-on-warning`). |
| 2 | CLI / config / backend setup error (the run never completed). |

**Worked example — single file, text output:**

```sh
sysml-validate validate model.sysml
# sysml-validate 0.15.0 — native backend, ruleset SYSML/0.5.0
# config: (none)   project: (no manifest)
# 2026-05-19T13:00:00Z
#
# model.sysml: 1 finding(s)
#   error  SYSML041 [3:5]  Duplicate member name 'Engine'.
```

**Worked example — CI, SARIF to stdout:**

```sh
sysml-validate validate src --ci > findings.sarif
echo "exit code: $?"
```

Upload `findings.sarif` to GitHub Advanced Security / SonarQube / etc.
Each result carries `partialFingerprints.diagnosticHash/v1` so
deduplication is stable across runs.

**Worked example — strict gate:**

```sh
sysml-validate validate src --strict --fail-on-warning
```

Promotes unresolved-reference warnings to part of the gate. Combine
with `[rules] SYSML040 = "error"` in `sysml-validate.toml` if the
project wants unresolved references treated as build-breaking
unconditionally.

### 4.3 sysml-validate grammar-info

Show the SysML v2 / KerML textual grammar references this build is
aligned to.

```sh
sysml-validate grammar-info
sysml-validate grammar-info --format json
```

Output lists filenames (`SysML-textual-bnf.kebnf`,
`KerML-textual-bnf.kebnf`), the OMG specification revisions they
track, and the on-disk path under `vendor/sysml-v2-release/bnf/`.

### 4.4 sysml-validate corpus-info

List public SysML v2 model corpora useful for smoke / differential
testing. Repo URLs and descriptions only; **no network calls are made**.

```sh
sysml-validate corpus-info
sysml-validate corpus-info --format json
```

### 4.5 sysml-validate library-info

Show the embedded standard library inventory: source description,
file count, declaration count, and indexed package names.

```sh
sysml-validate library-info
sysml-validate library-info --library-path /path/to/sysml.library
sysml-validate library-info --format json
```

`--library-path` shows what's in an on-disk library directory instead
of the embedded one — useful for diffing a pre-release OMG library
against the embedded copy.

### 4.6 sysml-validate lsp

Run the Language Server. Speaks LSP 3.x over stdin/stdout JSON-RPC.
**No flags.** Started by an editor client, not by a user at a
terminal — see §10.

### 4.7 Global options

| Flag | Description |
|---|---|
| `-h, --help` | Top-level help (subcommand table + global options + examples). |
| `-V, --version` | Print `sysml-validate <version>` and exit. |

---

## 5. Configuration Reference

### 5.1 sysml-validate.toml

`sysml-validate` discovers a `sysml-validate.toml` in the working
directory or any ancestor directory and loads it automatically. CLI
flags always override config values. Pass `--no-config` to skip
discovery; pass `--config <PATH>` to load an explicit file.

Supported keys:

| Key | Type | Effect |
|---|---|---|
| `project_root` | path string | Project source root; relative paths in `include`/`exclude` and in diagnostics are interpreted against this. |
| `default_format` | `"text"` \| `"plain"` \| `"json"` \| `"sarif"` \| `"junit"` | Default `--format`. |
| `default_strict` | bool | Default value of `--strict`. |
| `default_fail_on_warning` | bool | Default value of `--fail-on-warning`. |
| `include` | array of glob | Files to include (allowlist). Supports `*`, `**`, `?`. |
| `exclude` | array of glob | Files to exclude. Applied after `include`. |
| `[rules]` | table | Per-rule severity override: each key is a `SYSML*` code; value is `"error"`, `"warning"`, `"info"`, or `"off"`. |

Unknown keys are rejected at parse time (`deny_unknown_fields`), so
typos surface at validation time instead of silently doing nothing.

Example:

```toml
project_root = "."
default_format = "sarif"
default_strict = true
default_fail_on_warning = false

include = ["src/**/*.sysml", "src/**/*.kerml"]
exclude = ["**/target/**", "**/generated/**"]

[rules]
SYSML040 = "error"   # promote unresolved-reference warning to error
SYSML041 = "off"     # suppress duplicate-member-in-scope entirely
```

### 5.2 .project.json (Sysand manifest)

`sysml-validate` discovers a Sysand-compatible `.project.json` in the
working directory or any ancestor.

Fields parsed:

| Field | Type | Effect |
|---|---|---|
| `name` | string | Surfaced in the metadata block. |
| `version` | string | Surfaced in the metadata block. |
| `description` | string | Open passthrough. |
| `root` | path | Project source root. Takes precedence over `sysml-validate.toml`'s `project_root`. |
| `dependencies[]` | array | `{name, version, source}` entries. Parsed; the dependency-graph resolver is gated on Phase 2 KPAR support. |
| `meta` | open object | Passthrough for tool-specific metadata. |

Malformed manifests are rejected at parse time.

### 5.3 Configuration precedence

For each setting, the highest-precedence source wins:

1. **CLI flag** (`--format`, `--strict`, `--fail-on-warning`, `--config`, etc.).
2. **`sysml-validate.toml`** values (`default_format`, `default_strict`, etc.).
3. **Built-in defaults** (`--format text`, `--strict` off, `--fail-on-warning` off, `--timeout 60`).

For project root, the order is:

1. **`.project.json`** `root` field.
2. **`sysml-validate.toml`** `project_root` field.
3. **Directory** containing the discovered `sysml-validate.toml`.

### 5.4 Per-rule severity overrides

The `[rules]` table in `sysml-validate.toml` overrides individual rule
severities. Available values:

- `"error"` — emit as error (affects exit code).
- `"warning"` — emit as warning.
- `"info"` — emit as informational note (never affects exit code).
- `"off"` — suppress the rule entirely (the diagnostic is not emitted at all).

Severity overrides apply project-wide. Inline suppressions (§8) are
the right tool for one-off cases.

---

## 6. Output Formats

`sysml-validate` emits findings in five formats. All carry the same
underlying information; they differ in how they package it.

### 6.1 text (default)

Human-readable. Header with metadata block, one section per file with
findings grouped by severity. No ANSI escape sequences ([ACC]).

```
sysml-validate 0.15.0 — native backend, ruleset SYSML/0.5.0
config: ./sysml-validate.toml   project: my-project (0.1.0)
2026-05-19T13:00:00Z

src/Engine.sysml: 2 finding(s)
  error    SYSML041 [12:5]  Duplicate member name 'mass'.
  warning  SYSML040 [18:8]  Identifier reference not resolvable: 'PistonX'.

src/Wheel.sysml: 0 finding(s)
```

### 6.2 plain (Section 508 / IDE / grep)

GCC-style one-diagnostic-per-line. No header, no ANSI. Each line is a
single diagnostic in the format every IDE problem matcher already
understands:

```
src/Engine.sysml:12:5: error: SYSML041: Duplicate member name 'mass'.
src/Engine.sysml:18:8: warning: SYSML040: Identifier reference not resolvable: 'PistonX'.
```

Recommended for screen-reader users and for piping to `grep` /
`awk` / Vim's `:cgetfile`. See [ACC].

### 6.3 json (legacy)

Stable JSON shape with a `metadata` block and a `results` array.
Useful for ad-hoc tooling that doesn't speak SARIF.

```json
{
  "metadata": {
    "tool": {"name": "sysml-validate", "version": "0.15.0"},
    "rule_catalog": {"version": "0.5.0"},
    "invocation": {
      "timestamp_utc": "2026-05-19T13:00:00Z",
      "backend": "native",
      "strict": false,
      "format": "json"
    }
  },
  "results": [
    {
      "path": "src/Engine.sysml",
      "diagnostics": [
        {
          "severity": "error",
          "code": "SYSML041",
          "message": "Duplicate member name 'mass'.",
          "position": {"line": 12, "column": 5},
          "fingerprint": "1a2b3c4d5e6f7890"
        }
      ]
    }
  ]
}
```

### 6.4 sarif (CI default)

SARIF 2.1.0 — the OASIS standard for static-analysis findings.
Consumed without modification by GitHub Advanced Security, SonarQube,
GitLab Ultimate, Iron Bank, and Azure DevOps.

Every result carries:

- `ruleId`, `level`, `message.text`.
- `locations[0].physicalLocation.{artifactLocation, region}` with
  `file://` URIs (cross-platform; Windows paths are normalized).
- `partialFingerprints.diagnosticHash/v1` — a stable SHA-256-derived
  identifier used by baseline mode (§9) and consumers' dedup logic.
- `baselineState`: `new` or `unchanged` when run with `--baseline`.

Suppressed diagnostics appear as `results[].suppressions[]` entries
with `kind: "inSource"` and `status: "accepted"` (the SARIF-mandated
audit record).

The `runs[0].tool.driver.rules[]` array is the full rule catalog with
`id`, `name`, `shortDescription`, `fullDescription`,
`defaultConfiguration.level`, and `helpUri`.

### 6.5 junit (Jenkins / GitLab)

Maven Surefire-compatible JUnit XML. One `<testsuite>` per validated
file. Each diagnostic becomes a `<testcase>`. Errors render as
`<failure>`. Warnings render as `<system-out>` (or `<error>` under
`--fail-on-warning`). Suppressed diagnostics are excluded from the
testcase count.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="sysml-validate" tests="1" failures="1" errors="0">
  <testsuite name="src/Engine.sysml" tests="1" failures="1" errors="0" skipped="0" time="0">
    <testcase classname="src/Engine.sysml" name="SYSML041:12:5 (0)" time="0">
      <failure type="SYSML041" message="Duplicate member name &apos;mass&apos;.">Duplicate member name 'mass'.</failure>
    </testcase>
  </testsuite>
</testsuites>
```

Drop into Jenkins' "Publish JUnit test result report" or GitLab CI's
`reports.junit` stanza.

---

## 7. Rule Catalog

Rule IDs use the namespace `SYSMLxxx`. Every diagnostic emitted by
`sysml-validate` has a stable code from this catalog. The current
catalog version (`RULE_CATALOG_VERSION`) is **0.5.0**.

When a rule's *meaning* changes, the catalog version is bumped so
consumers can gate baselines on version.

### 7.1 Lexical rules (SYSML00x – SYSML01x)

| Code | Severity | Detects | Remediation |
|---|---|---|---|
| `SYSML001` | error | Invalid control character in source text | Strip the offending byte; ensure the file is UTF-8 |
| `SYSML002` | error | Unterminated string literal | Close the string with `"` |
| `SYSML003` | error | Unterminated block comment | Close with `*/` |
| `SYSML010` | error | Unsupported file extension | Rename to `.sysml` or `.kerml`, or drop from include patterns |
| `SYSML012` | error | Unable to read file as UTF-8 text | Re-save as UTF-8 |

### 7.2 Delimiter rules (SYSML02x)

| Code | Severity | Detects | Remediation |
|---|---|---|---|
| `SYSML020` | error | Unmatched closing delimiter | Remove the extra `)` / `}` / `]` |
| `SYSML021` | error | Unclosed delimiter | Add the matching closer |

### 7.3 Statement-shape rules (SYSML03x)

| Code | Severity | Detects | Remediation |
|---|---|---|---|
| `SYSML030` | error | Expected `package` after `library` | Use `library package <Name>` |
| `SYSML031` | error | Alias declaration must include `for` | `alias <New> for <Existing>;` |
| `SYSML032` | error | Dependency missing supplier after `to` | `dependency <Name> to <Supplier>;` |
| `SYSML033` | error | Usage missing declared name or specialization | Provide a name (`part x;`) or specialization (`part :> Y;`) |
| `SYSML034` | error | Definition missing a name | `part def Engine`, not `part def` |
| `SYSML035` | error | Missing `;` or `{` terminator | Terminate the statement |

### 7.4 Reference / scope rules (SYSML04x)

| Code | Severity | Detects | Remediation |
|---|---|---|---|
| `SYSML040` | warning | Identifier reference not resolvable | Add an `import`; check spelling; verify the project root captures all source files |
| `SYSML041` | error | Duplicate member name in lexical scope | Rename one of the conflicting members |

### 7.5 Structural rules (SYSML2xx)

| Code | Severity | Detects | Remediation |
|---|---|---|---|
| `SYSML100` | warning | tree-sitter parser could not understand region (only under `--strict` until grammar coverage improves) | Inspect the flagged region; report a false positive if the syntax is legal |
| `SYSML210` | error | `:>` / `specializes` target does not resolve | Add an `import`; correct the target name |
| `SYSML211` | error | `:>>` / `redefines` target does not resolve | Add an `import`; correct the target name |
| `SYSML212` | error | Feature specializes itself | Specialize a different feature, or remove the `:>` |
| `SYSML213` | error | Feature redefines itself | Redefine a different feature, or remove the `:>>` |
| `SYSML220` | error | Specialization graph contains a cycle (project-wide) | Break the cycle; the message names the full path |

### 7.6 Suppression-directive rules (SYSML05x – SYSML06x)

| Code | Severity | Detects | Remediation |
|---|---|---|---|
| `SYSML050` | warning | Suppression directive did not match any diagnostic | Remove the dead directive |
| `SYSML060` | warning | Suppression directive has invalid syntax | See §8 for correct syntax |

### 7.7 Configuration / setup rules (SYSML8xx)

| Code | Severity | Detects | Remediation |
|---|---|---|---|
| `SYSML800` | error | Configuration file is invalid | Inspect `sysml-validate.toml`; the message names the offending key |

### 7.8 Official backend rules (SYSML9xx)

| Code | Severity | Detects | Remediation |
|---|---|---|---|
| `SYSML900` | error | `--official-command` parse/setup error | Check the argv template; ensure `{file}` is present |
| `SYSML901` | error | Official validator could not be executed | Verify the executable exists and is on PATH |
| `SYSML902` | error | Official validator returned non-zero exit status | Inspect the official validator's stderr |
| `SYSML903` | info | Official validator returned informational output | No action; the output is preserved |
| `SYSML904` | error | Official validator exceeded `--timeout` | Increase `--timeout`, or investigate why the child hung |

---

## 8. Suppressions

`sysml-validate` honors inline suppression directives in source
comments. Suppressed diagnostics are kept on the result list (marked
with `suppression`), excluded from text and JSON output by default,
always present in SARIF as `suppressions[]` entries, and **never
affect the exit code**.

### 8.1 Inline directive syntax

```
// sysml-validate: disable=SYSML041
// sysml-validate: disable=SYSML041,SYSML040
// sysml-validate: disable-next-line=SYSML041
// sysml-validate: disable=all
```

| Directive | Scope |
|---|---|
| `disable=<CODE>` | Same line as the directive. |
| `disable=<CODE1>,<CODE2>,…` | Same line; multiple codes. |
| `disable-next-line=<CODE>` | Next *non-blank* line. |
| `disable=all` | Same line; every rule. |

### 8.2 Examples

```sysml
package P {
  // sysml-validate: disable=SYSML041
  part def Engine; part def Engine;        // duplicate suppressed

  // sysml-validate: disable-next-line=SYSML040
  part wheel :> Missing;                   // unresolved-ref suppressed

  // sysml-validate: disable=all
  alias E for Engine;                      // every rule suppressed
}
```

### 8.3 Dead-directive warnings

A directive that doesn't match any diagnostic produces `SYSML050`
(warning) so dead directives surface and can be cleaned up. An
invalid directive form produces `SYSML060`.

### 8.4 Showing suppressed diagnostics

```sh
sysml-validate validate src --show-suppressed
```

Includes suppressed entries in text and JSON output. SARIF includes
them unconditionally as `suppressions[]` entries with
`kind: "inSource"` (the SARIF audit record).

---

## 9. Baseline / Diff Workflow

Baseline mode is the right tool when adopting `sysml-validate` on a
large existing project: record the current set of findings as the
accepted baseline, then in subsequent runs only **new** findings fail
the build.

### 9.1 Seeding a baseline

```sh
sysml-validate validate src --ci --baseline baseline.sarif --update-baseline
```

- Runs validation.
- Writes the resulting SARIF to `baseline.sarif`.
- Forces exit 0 (the whole point — the current state is the new floor).

Commit `baseline.sarif` to the repo.

### 9.2 Subsequent runs

```sh
sysml-validate validate src --ci --baseline baseline.sarif > findings.sarif
```

- Loads `baseline.sarif`.
- Walks the current findings and matches each by `(ruleId,
  diagnosticHash/v1)` against the baseline.
- Sets `baselineState: "unchanged"` on matches, `baselineState: "new"`
  on misses.
- Exit 1 iff any *new* finding is an error (or a warning under
  `--fail-on-warning`).

### 9.3 Updating the baseline

Re-run with `--update-baseline` whenever the team explicitly accepts
the current finding set as the new floor:

```sh
sysml-validate validate src --ci --baseline baseline.sarif --update-baseline
```

This is also how you re-seed after a major refactor.

### 9.4 Fingerprints

Findings are matched by `partialFingerprints.diagnosticHash/v1`, a
SHA-256-derived hash of `(rule code, normalized path, genericized
message)`. The hash is intentionally **position-independent** — moving
a finding to a different line does not invalidate the baseline.

---

## 10. LSP Integration

`sysml-validate lsp` starts a Language Server speaking LSP 3.x over
stdin/stdout JSON-RPC. The server is **launched by an editor client**,
not by a user at a terminal — running it from a shell will appear to
hang because it is waiting for a JSON-RPC `initialize` request on
stdin.

### 10.1 Capabilities

| LSP method | Behavior |
|---|---|
| `initialize` / `initialized` | Standard handshake; server advertises its capabilities. |
| `textDocument/didOpen` | Parse + validate the buffer, publish diagnostics. |
| `textDocument/didChange` (full-text sync) | Update the buffer, re-validate, re-publish diagnostics. |
| `textDocument/didClose` | Clear diagnostics for the file. |
| `textDocument/publishDiagnostics` | Server-initiated push of diagnostics to the client. |
| `textDocument/hover` | Render the rule catalog entry for the diagnostic under the cursor as Markdown. |
| `shutdown` / `exit` | Standard shutdown. |

### 10.2 VS Code

Install any "generic LSP client" extension (e.g., **vscode-languageclient** or **Mark.ts**), then add to your VS Code settings:

```json
{
  "languageserver": {
    "sysml-validate": {
      "command": "sysml-validate",
      "args": ["lsp"],
      "filetypes": ["sysml", "kerml"],
      "rootPatterns": [".project.json", "sysml-validate.toml", ".git/"]
    }
  }
}
```

### 10.3 Neovim (nvim-lspconfig)

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.sysml_validate then
  configs.sysml_validate = {
    default_config = {
      cmd = { 'sysml-validate', 'lsp' },
      filetypes = { 'sysml', 'kerml' },
      root_dir = lspconfig.util.root_pattern('.project.json', 'sysml-validate.toml', '.git'),
      settings = {},
    },
  }
end
lspconfig.sysml_validate.setup{}
```

### 10.4 Helix

In `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "sysml"
scope = "source.sysml"
file-types = ["sysml"]
language-servers = ["sysml-validate"]

[language-server.sysml-validate]
command = "sysml-validate"
args = ["lsp"]
```

### 10.5 Emacs (lsp-mode)

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(sysml-mode . "sysml"))
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("sysml-validate" "lsp"))
    :activation-fn (lsp-activate-on "sysml")
    :server-id 'sysml-validate)))
```

---

## 11. CI/CD Integration Patterns

### 11.1 GitHub Actions

```yaml
name: SysML validation
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install sysml-validate
        run: |
          curl -LO https://github.com/<owner>/sysml-cli/releases/download/v0.15.0/sysml-validate-0.15.0-x86_64-unknown-linux-gnu.tar.gz
          tar -xzf sysml-validate-0.15.0-x86_64-unknown-linux-gnu.tar.gz
          sudo cp sysml-validate-0.15.0-x86_64-unknown-linux-gnu/bin/sysml-validate /usr/local/bin/
      - name: Validate
        run: sysml-validate validate src --ci > findings.sarif
      - name: Upload SARIF
        if: always()
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: findings.sarif
```

### 11.2 GitLab CI

```yaml
sysml-validate:
  image: registry.example.com/sysml-validate:0.15.0
  script:
    - sysml-validate validate src --format junit > junit.xml
  artifacts:
    when: always
    reports:
      junit: junit.xml
```

### 11.3 Jenkins (declarative)

```groovy
pipeline {
    agent any
    stages {
        stage('SysML validate') {
            steps {
                sh 'sysml-validate validate src --format junit > junit.xml'
            }
            post {
                always {
                    junit 'junit.xml'
                }
            }
        }
    }
}
```

### 11.4 Azure DevOps

```yaml
- script: |
    sysml-validate validate src --ci > $(Build.ArtifactStagingDirectory)/findings.sarif
  displayName: 'Validate SysML models'

- task: PublishBuildArtifacts@1
  condition: always()
  inputs:
    pathToPublish: '$(Build.ArtifactStagingDirectory)/findings.sarif'
    artifactName: 'sarif'
```

### 11.5 Iron Bank / Platform One

Iron Bank consumes SARIF via Anchore Enterprise and CodeQL pipelines.
Output `--format sarif` and upload as a job artifact; the platform's
ingestion job will surface findings in the central dashboard.

### 11.6 Baseline-driven pipelines

For projects adopting the tool on a non-clean codebase, combine the
above with the baseline workflow (§9). Commit `baseline.sarif` to the
repo; the CI step becomes:

```yaml
- run: sysml-validate validate src --ci --baseline baseline.sarif > findings.sarif
```

CI fails only on **new** findings; reviewers see existing findings as
`baselineState: "unchanged"` in the SARIF.

---

## 12. Troubleshooting

### 12.1 "no .sysml or .kerml files found"

You ran `sysml-validate validate <path>` against a directory that
contains no model files (or whose model files were filtered out by
include / exclude globs).

- Check that the path is correct.
- Check `sysml-validate.toml`'s `include` / `exclude` patterns.
- Confirm the files have `.sysml` or `.kerml` extensions.

### 12.2 SYSML040 fires for a name that *is* defined

Likely causes, in order:

1. **The defining file isn't in the validation run.** The project
   index is built from the files passed to `validate`. Pass the whole
   project directory, not a single file.
2. **The defining file is under a different `project_root`.** Check
   `.project.json` `root` and `sysml-validate.toml` `project_root`.
3. **The name is in a package that isn't imported.** Add `import
   <Package>::*;` or qualify the reference.
4. **The name is in the standard library and you didn't pass
   `--strict`.** Library resolution only runs under `--strict`.

### 12.3 Official backend hangs

```sh
sysml-validate validate model.sysml --backend official \
  --official-command "some-validator {file}" --timeout 60
```

- The child is killed after `--timeout` seconds with `SYSML904`.
- If the timeout is firing, increase it OR inspect why the child
  hung — typically it's waiting on stdin or on a network call.
- The child's stdout and stderr are drained in worker threads, so a
  chatty child cannot block on a full pipe.

### 12.4 Baseline classifies everything as `new` on a clean re-run

Causes:

1. **Project root drift.** If the working directory changed between
   the seeding run and the verifying run, paths in the fingerprint
   may differ. Run both from the project root, or pass `project_root`
   in `sysml-validate.toml`.
2. **Rule catalog bump.** If `RULE_CATALOG_VERSION` changed, a rule
   might emit a different message template, changing fingerprints.
   Re-seed the baseline with `--update-baseline`.
3. **Message-text refactor in your own model.** Position is excluded
   from fingerprints, but the *identifier text* in the diagnostic is
   genericized only for certain templates; if a rule's message
   embeds a name and the name changed, the fingerprint changes.
   Re-seed if intentional.

### 12.5 SARIF upload to GitHub Advanced Security is rejected

GHAS validates SARIF against its schema. Typical rejection causes:

1. Output didn't include the rule catalog. Confirm
   `runs[0].tool.driver.rules` is populated.
2. The artifact location is a relative path that GHAS can't map. Set
   `project_root` so paths in the SARIF are repository-relative.
3. The file is too large. GHAS has a 10 MB / 1 MB-per-result limit.
   Use baseline mode or `[rules] SYSML040 = "off"` to bring the
   finding count down.

### 12.6 LSP server "hangs" when launched from a terminal

Expected. The server is waiting for a JSON-RPC `initialize` request
on stdin. It is meant to be started by an editor client (§10), not by
a human at a shell prompt.

### 12.7 Reproducible-build verification fails (diffoscope reports diff)

The release workflow's `reproducibility-check` job will fail the
release on any diff. To debug:

1. Read the diffoscope output — it names the sections that differ.
2. The usual culprits are debug info containing build-host paths
   (mitigated by `--remap-path-prefix` in `.cargo/config.toml`),
   embedded timestamps (mitigated by `SOURCE_DATE_EPOCH`), and
   non-deterministic codegen ordering (mitigated by
   `codegen-units = 1`).
3. Reproduce locally: rebuild the same tag on a clean checkout and
   compare with `diffoscope target/release/sysml-validate
   /path/to/released/binary`.
4. See [REP] for the canonical recipe.

### 12.8 Tree-sitter grammar warnings (SYSML100)

`SYSML100` fires when the embedded tree-sitter SysML grammar cannot
parse a region of source. Today the grammar (`tree-sitter-sysml`
0.1) is incomplete on rare shapes. `SYSML100` is **gated behind
`--strict`** for that reason. If you hit a SYSML100 on syntax you
believe is legal, it is likely a grammar-coverage gap, not a bug in
your model. File an issue with the offending snippet.

---

## 13. Security & Trust Verification

The release bundle ships with five trust artifacts. Verify them in
this order. Each is independent — a verified SHA-256 alone is not
sufficient; the chain matters.

### 13.1 SHA-256

```sh
sha256sum -c sysml-validate-<target>.sha256
# sysml-validate-<target>: OK
```

Confirms the bytes match what the release workflow produced. Does
*not* prove provenance.

### 13.2 Sigstore (cosign)

```sh
cosign verify-blob \
  --bundle sysml-validate-<target>.cosign.bundle \
  --certificate-identity-regexp 'https://github\.com/<owner>/sysml-cli/' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  sysml-validate-<target>
```

Confirms the artifact was signed by a workflow on the project's
GitHub repo. The Rekor transparency-log inclusion proof is in the
bundle, so verification is offline once the Sigstore root is trusted.

### 13.3 GPG

```sh
gpg --verify sysml-validate-<target>.asc sysml-validate-<target>
```

Confirms the long-lived offline key signed the artifact. The
fingerprint is published in [SEC].

### 13.4 SLSA v1.0 Build L3 provenance

```sh
gh attestation verify sysml-validate-<target> --owner <owner>
```

Confirms the build occurred on a SLSA L3 platform (GitHub-hosted
runner) with the documented build steps and source revision.

### 13.5 SBOM

```sh
# Inspect with the tool of your choice
grype sysml-validate-<target>.cdx.json
trivy sbom sysml-validate-<target>.cdx.json
```

Both CycloneDX 1.6 and SPDX 3.0 are emitted per release; pick the one
your scanner prefers. NTIA Minimum Elements (component, version,
supplier, dependency, hash, license, time of generation, author) are
included.

### 13.6 Reproducible build

See [REP]. Independent rebuild produces a byte-identical binary; the
release workflow runs `diffoscope --exit-code` between two builds and
fails the release on any difference.

### 13.7 Verifying the release bundle as a whole

```sh
sha256sum -c BUNDLE-MANIFEST.txt
```

`BUNDLE-MANIFEST.txt` lists every file in the bundle with its SHA-256;
this command verifies the whole tree at once.

---

## 14. Compliance Reference

See [`compliance/INDEX.md`](compliance/INDEX.md). It maps reviewer
roles (SCRM, ATO, airworthiness, NASA, CMMC L2, Section 508) to the
specific document in this bundle that answers their questions.

Inventory:

- [`compliance/ssdf-mapping.md`](compliance/ssdf-mapping.md) — NIST SP 800-218
- [`compliance/nist-800-53-mapping.md`](compliance/nist-800-53-mapping.md)
- [`compliance/cmmc-l2-deployment.md`](compliance/cmmc-l2-deployment.md)
- [`compliance/do-330-qualification-kit/`](compliance/do-330-qualification-kit/) — DAL A–D, TQL-5
- [`compliance/nasa-npr-7150-2d-tool-validation.md`](compliance/nasa-npr-7150-2d-tool-validation.md) — Class A/B/C
- [`accessibility.md`](accessibility.md) — Section 508 / VPAT 2.5 draft

---

## 15. Glossary

| Term | Definition |
|---|---|
| **AST** | Abstract Syntax Tree. The tree-sitter-produced structure used by some validators. |
| **ATO** | Authority to Operate. The accreditation by a system owner that admits a tool to operate on the system. |
| **Baseline** | A SARIF log treated as the accepted floor of findings. Subsequent runs classify each finding as `new` or `unchanged` against it. |
| **CMMC** | Cybersecurity Maturity Model Certification. DoD's compliance regime for contractors handling CUI. |
| **CycloneDX** | OWASP SBOM standard. Version 1.6 in use. |
| **DAL** | Design Assurance Level (A–E). DO-178C software safety classification. |
| **DO-178C / DO-330** | RTCA standards for airborne software (DO-178C) and tool qualification (DO-330). |
| **Fingerprint** | A SHA-256-derived stable identifier per diagnostic. Used for baseline matching and consumer deduplication. |
| **KerML** | The OMG Kernel Modeling Language. The foundation `SysML v2` is built on. |
| **LSP** | Language Server Protocol. The JSON-RPC editor-tooling protocol Microsoft introduced and the industry adopted. |
| **MBSE** | Model-Based Systems Engineering. The discipline `sysml-validate` serves. |
| **NIST 800-53** | The Federal control catalog. Revision 5 in use. |
| **NIST SP 800-218** | Secure Software Development Framework (SSDF). |
| **NPR 7150.2D** | NASA's software engineering procedural requirements. |
| **OMG** | Object Management Group. Custodian of the SysML / KerML / UML / Systems Modeling API specifications. |
| **Pilot Implementation** | The OMG's reference implementation of SysML v2 / KerML. Java / Xtext stack. |
| **SARIF** | Static Analysis Results Interchange Format. OASIS standard 2.1.0. |
| **SBOM** | Software Bill of Materials. CycloneDX or SPDX. |
| **Sigstore / cosign** | Keyless-OIDC code-signing system; signatures are logged in the public Rekor transparency log. |
| **SLSA** | Supply-chain Levels for Software Artifacts. v1.0 Build L3 in use. |
| **SPDX** | ISO/IEC 5962 SBOM standard. Version 3.0 in use. |
| **SSDF** | Secure Software Development Framework. NIST SP 800-218. |
| **Sysand** | Sensmetry's Python package-manager for SysML v2. The `.project.json` manifest comes from there. |
| **SysML v2** | OMG Systems Modeling Language version 2. Successor to SysML 1.x; formally specified, not a UML profile. |
| **TQL** | Tool Qualification Level (TQL-1 through TQL-5). DO-330 grading. |
| **VPAT** | Voluntary Product Accessibility Template. The Section 508 self-conformance statement. |

---

## Appendix A. Sample configurations

### A.1 Minimal `sysml-validate.toml`

```toml
project_root = "."
default_format = "sarif"
default_strict = true
```

### A.2 Strict CI gate

```toml
project_root = "."
default_format = "sarif"
default_strict = true
default_fail_on_warning = true

include = ["src/**/*.sysml", "src/**/*.kerml"]
exclude = ["**/target/**", "**/generated/**", "**/vendor/**"]

[rules]
SYSML040 = "error"          # unresolved references break the build
```

### A.3 Lenient adoption mode

For a project on first contact with the tool: emit a baseline, accept
the current state, only fail on net-new findings.

```toml
project_root = "."
default_format = "sarif"
default_strict = false
default_fail_on_warning = false
```

Then:

```sh
sysml-validate validate src --ci --baseline baseline.sarif --update-baseline
```

### A.4 Minimal `.project.json`

```json
{
  "name": "my-program",
  "version": "0.1.0",
  "description": "My SysML v2 model",
  "root": "src"
}
```

---

## Appendix B. Air-gap deployment recipe

For a DoD IL4/IL5 enclave or any disconnected environment:

1. **On a connected build host:**
   ```sh
   git clone --recurse-submodules https://github.com/<owner>/sysml-cli
   cd sysml-cli
   cargo vendor                  # captures crate sources into ./vendor/
   tar -czf sysml-cli-source.tar.gz .
   ```
2. **Or pull a signed release bundle:**
   ```sh
   curl -LO https://github.com/<owner>/sysml-cli/releases/download/v0.15.0/sysml-validate-0.15.0-x86_64-unknown-linux-gnu.tar.gz
   # plus the .sha256, .cosign.bundle, .asc files
   ```
3. **Transfer to the enclave** via the approved cross-domain
   solution.
4. **Verify** per §13 before extracting / executing.
5. **Deploy** the binary to the operator path (typically
   `/usr/local/bin` or an organization-approved equivalent).
6. **No runtime network access** is required after install.

See [OFF] for the canonical statement of the per-subcommand network
surface (every subcommand: zero).

---

## Appendix C. Document conventions

- **File paths** in this document use forward slashes regardless of
  host OS. The tool normalizes path separators in diagnostic output.
- **CLI invocations** use POSIX shell syntax. On Windows, replace
  line-continuation `\` with backtick `` ` `` in PowerShell or `^` in
  `cmd.exe`.
- **Reference tags** in square brackets (e.g., `[SEC]`) refer to the
  reference table in §1.4.
- **Diagnostic codes** use the namespace `SYSMLxxx`. The catalog
  version is bumped when a rule's meaning changes; consumers can gate
  baselines on the catalog version.

---

*End of manual.*
