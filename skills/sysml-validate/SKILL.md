---
name: sysml-validate
description: Validate SysML v2 and KerML textual models with the sysml-validate CLI. Use when the user is editing, reviewing, or running CI against .sysml or .kerml files; when interpreting SYSMLxxx diagnostic codes; when writing or fixing baselines, suppression directives, or sysml-validate.toml configuration; when integrating the validator into CI/CD pipelines; or when verifying release artifacts (SHA-256, cosign, GPG, SLSA, SBOM).
license: MIT
compatibility: Requires the sysml-validate binary on PATH (v0.15.0+). No network access required for any validation operation.
metadata:
  project: sysml-cli
  project-url: https://github.com/Systems-Modeling/SysML-v2-Release
  spec-version: agentskills.io/specification
  audience: agents assisting government MBSE programs, defense primes, NASA, federal civilian
---

# sysml-validate skill

You are assisting a user who works with **SysML v2** or **KerML** textual
models. `sysml-validate` is a Rust CLI preflight validator for those
languages. This skill teaches you how to invoke it, interpret its
findings, and integrate it into the user's workflow.

## When to activate this skill

Activate when **any** of the following is true:

- The user is editing a `.sysml` or `.kerml` file.
- The user mentions SysML v2, KerML, MBSE, the OMG Pilot Implementation,
  or Sysand.
- The user is configuring CI/CD that should validate SysML/KerML models.
- The user references a diagnostic code matching `SYSML\d{3}` (e.g.,
  `SYSML041`, `SYSML220`).
- The user mentions `sysml-validate`, `sysml-validate.toml`, or
  `.project.json`.
- The user is preparing a tagged release of a SysML/KerML project.

## What to do — by user intent

### Intent: "Check / validate / lint my model"

Run the native backend:

```sh
sysml-validate validate <path>
```

Where `<path>` is a file or directory. Directories are walked
recursively. If the user is in a project directory, just pass `.` or
the source root. Default output is human-readable text.

For an editor / IDE / grep-friendly variant:

```sh
sysml-validate validate <path> --format plain
```

This emits GCC-style `path:line:col: severity: code: message`.

### Intent: "Fail my CI on validation errors"

```sh
sysml-validate validate <src> --ci > findings.sarif
```

`--ci` is shorthand for `--format sarif`. Exit code is `0` if no
errors, `1` if errors found, `2` on setup error. Upload `findings.sarif`
to the consumer (GitHub Advanced Security, SonarQube, etc.).

For strict gates that also fail on warnings:

```sh
sysml-validate validate <src> --strict --fail-on-warning
```

For JUnit XML (Jenkins / GitLab):

```sh
sysml-validate validate <src> --format junit > junit.xml
```

See [references/cicd-recipes.md](references/cicd-recipes.md) for
per-platform CI recipes (GitHub Actions, GitLab CI, Jenkins, Azure
DevOps, Iron Bank).

### Intent: "Adopt the validator on an existing project (don't fail on legacy findings)"

Seed a baseline once, commit it, then in CI only **new** findings fail:

```sh
# One-time seeding
sysml-validate validate <src> --ci --baseline baseline.sarif --update-baseline

# Subsequent CI invocations
sysml-validate validate <src> --ci --baseline baseline.sarif
```

`--update-baseline` requires both `--baseline <path>` and `--format
sarif` (or `--ci`). Commit `baseline.sarif` to the repo.

### Intent: "Interpret / fix a SYSMLxxx diagnostic"

See [references/rule-catalog.md](references/rule-catalog.md) for the
complete code → meaning → remediation table.

Quick remediation map for the rules that show up most:

| Code | TL;DR fix |
|---|---|
| `SYSML020` / `SYSML021` | Balance the `{` / `}` / `(` / `)` / `[` / `]`. |
| `SYSML033` | Provide a name or a specialization: `part x;` or `part :> Y;`. |
| `SYSML034` | Give the definition a name: `part def Engine`, not `part def`. |
| `SYSML035` | Terminate with `;` or `{`. |
| `SYSML040` | Add an `import`, fix spelling, or pass `--strict` to be sure the standard library was consulted. |
| `SYSML041` | Rename one of the duplicate members. |
| `SYSML210` / `SYSML211` | `:>` / `:>>` target not found — add `import`, correct the name. |
| `SYSML212` / `SYSML213` | Feature is specializing/redefining itself — remove the `:>` / `:>>` or point at a different feature. |
| `SYSML220` | Specialization cycle — the message names the full cycle path; break it. |
| `SYSML904` | Official backend exceeded `--timeout`. Increase it or investigate why the child hung. |

### Intent: "Suppress a specific diagnostic on this line"

Use an inline comment directly above or on the offending line. Exact
syntax — **do not invent variations**:

```
// sysml-validate: disable=SYSML041
// sysml-validate: disable=SYSML041,SYSML040
// sysml-validate: disable-next-line=SYSML041
// sysml-validate: disable=all
```

See [references/suppression-syntax.md](references/suppression-syntax.md)
for scope semantics, dead-directive warnings (`SYSML050`), and invalid-
directive warnings (`SYSML060`).

### Intent: "Configure project-wide settings"

Create `sysml-validate.toml` at the project root (or any ancestor of
the working directory). Minimal example:

```toml
project_root = "."
default_format = "sarif"
default_strict = true

include = ["src/**/*.sysml", "src/**/*.kerml"]
exclude = ["**/target/**", "**/generated/**"]

[rules]
SYSML040 = "error"   # promote unresolved-ref warning to error
SYSML041 = "off"     # suppress the rule entirely (rare; prefer inline suppressions)
```

Per-rule severity values: `"error"`, `"warning"`, `"info"`, `"off"`.
Unknown keys are rejected at parse time, so typos surface immediately.

For Sysand-compatible project manifests, use `.project.json`:

```json
{
  "name": "my-program",
  "version": "0.1.0",
  "root": "src"
}
```

`.project.json` `root` takes precedence over `sysml-validate.toml`
`project_root`.

### Intent: "Use the official OMG Pilot for deeper checks"

Delegate to it via the hardened official backend:

```sh
sysml-validate validate <file> --backend official \
  --official-command "sysml-validator --strict {file}" \
  --timeout 120
```

`{file}` is replaced with each model path. The command is **argv-
invoked, not shell-invoked** — shell metacharacters survive only as
literal argv content. If the child exceeds `--timeout` seconds it is
terminated and `SYSML904` is emitted.

### Intent: "Set up editor integration"

Run the LSP server from the editor's LSP client. Sample configs:

- Neovim (nvim-lspconfig): see TECH_MANUAL.md §10.3
- VS Code: see TECH_MANUAL.md §10.2
- Helix: see TECH_MANUAL.md §10.4
- Emacs (lsp-mode): see TECH_MANUAL.md §10.5

Invoke as `sysml-validate lsp`. **No flags.** It speaks LSP 3.x over
stdio JSON-RPC.

### Intent: "Verify the release artifact"

Run all five checks before deploying a binary:

1. **SHA-256:** `sha256sum -c sysml-validate-<target>.sha256`
2. **cosign keyless OIDC:** `cosign verify-blob --bundle
   sysml-validate-<target>.cosign.bundle --certificate-identity-regexp
   'https://github\.com/<owner>/sysml-cli/' --certificate-oidc-issuer
   https://token.actions.githubusercontent.com sysml-validate-<target>`
3. **GPG:** `gpg --verify sysml-validate-<target>.asc
   sysml-validate-<target>`
4. **SLSA v1.0 Build L3:** `gh attestation verify sysml-validate-<target>
   --owner <owner>`
5. **Whole-bundle:** `sha256sum -c BUNDLE-MANIFEST.txt` (if the user
   pulled the structured `.tar.gz` / `.zip` bundle)

Don't skip any of them in a government context — each verifies a
different property.

## Don'ts

These are anti-patterns. Watch for them and steer the user (or
yourself) away.

- **Don't start `sysml-validate lsp` from a terminal** and wait for
  output. The LSP server is meant to be launched by an editor client.
  Started from a shell, it will appear to hang because it's waiting
  for a JSON-RPC `initialize` request on stdin. If the user did this
  by accident, the symptom is "the command isn't doing anything" —
  tell them to Ctrl+C and configure their editor's LSP client.

- **Don't propose line-number-based baseline diffs.** Baseline mode
  matches findings by `(ruleId, partialFingerprints.diagnosticHash/v1)`
  — a position-independent SHA-256-derived fingerprint. Moving a
  finding to a different line does NOT invalidate the baseline.

- **Don't suggest `--features fips`.** US-307 (`--fips`) is formally
  deferred in this project (see PRD §10 / US-307); the tool has no
  runtime cryptographic operations, so the flag would be a no-op. If
  the user asks for FIPS, explain that there is nothing to validate
  in FIPS mode today and point at the decision record.

- **Don't invent suppression-directive syntax.** The directives are
  `// sysml-validate: disable=<CODE>`, `disable=<CODE1>,<CODE2>`,
  `disable-next-line=<CODE>`, or `disable=all`. Anything else
  produces `SYSML060` (invalid syntax) — see references/.

- **Don't pass `--update-baseline` without `--ci` (or `--format
  sarif`) and `--baseline`.** The parser will reject it. Always
  supply all three together.

- **Don't add `# sysml-validate:`-style comments.** SysML/KerML
  line comments are `//`, not `#`. The `#` character introduces
  metadata tag declarations and will be parsed as such.

- **Don't suggest editing `vendor/sysml-v2-release/`.** That's a
  pinned git submodule of the OMG release. Any local edit will be
  lost on the next `git submodule update`, and a non-pinned library
  breaks reproducible builds.

## Exit-code contract

Memorize this — agents propose CI logic based on it:

| Exit code | Meaning |
|---|---|
| `0` | No errors. Build proceeds. |
| `1` | Validation errors found (or warnings with `--fail-on-warning`). Build fails. |
| `2` | CLI / config / backend setup error — the run never completed. **Investigate before retrying.** |

Treat exit `2` as fundamentally different from exit `1`: `1` is "your
model has errors," `2` is "the tool itself couldn't run." Don't lump
them.

## Output-format selection

| Goal | Format |
|---|---|
| Human reading | `text` (default) |
| Screen-reader / IDE problem-matcher / grep | `plain` |
| Legacy / scripts | `json` |
| CI (GitHub, GitLab, SonarQube, Iron Bank, Azure DevOps) | `sarif` (via `--ci`) |
| Jenkins / GitLab test reporters | `junit` |

When in doubt for CI, default to `sarif`. It is the format that
carries the most consumer-side semantics (fingerprints, suppressions,
baseline states, full rule catalog).

## Reference files

- [references/rule-catalog.md](references/rule-catalog.md) — every
  `SYSMLxxx` code with remediation guidance
- [references/suppression-syntax.md](references/suppression-syntax.md)
  — directive grammar, scope semantics, dead-directive rules
- [references/cicd-recipes.md](references/cicd-recipes.md) —
  per-platform integration recipes

For human-audience material:

- `docs/TECH_MANUAL.md` — full operator's manual
- `docs/EXECUTIVE_SUMMARY.md` — Flag/SES one-pager
- `docs/PRD-government-readiness.md` — full requirement set
- `docs/compliance/INDEX.md` — government-conformance reviewer index
- `docs/SECURITY.md` — release verification, vulnerability disclosure

## Defaults to use when unspecified

If the user doesn't specify, prefer:

- `--format text` for interactive use; `--format sarif` (`--ci`) for CI
- `--strict` **off** by default (it's a stricter gate; turn on when the
  user asks for it or wants unresolved-reference warnings)
- `--fail-on-warning` **off** by default (warnings shouldn't break the
  build unless the project has opted in)
- `--timeout 60` is the default for the official backend; raise only
  if the user reports legitimate slow validations
