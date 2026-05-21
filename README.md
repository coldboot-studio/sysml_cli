# sysml-validate

> Fast, deterministic SysML v2 validation for **agentic CI loops** —
> one self-contained binary, no JVM, no network, no telemetry.
> Structured findings in every format your pipeline (or your coding
> agent) already speaks.

`sysml-validate` exists to close a specific gap: when a coding agent
generates or modifies a [SysML v2](https://www.omg.org/spec/SysML/)
or KerML textual model, you need a fast, scriptable, headless
validator that the agent can drive itself and the human reviewer can
gate the PR on. The OMG reference implementations are excellent at
deep semantic conformance but are JVM-and-GUI-heavy; they're not
what fits inside an agent's tool loop. This is.

The native backend catches structural and reference errors in tens
of milliseconds against a single file or an entire project tree.
Deep semantic conformance (constraint evaluation, full OCL
well-formedness) delegates to the OMG Pilot via a hardened
`--backend official` channel when an adopter needs it.

## Why this exists

| The gap | What sysml-validate provides |
|---|---|
| Modeling tools assume an interactive desktop session | A CLI that runs in CI and inside agentic tool loops |
| OMG Pilot starts cold in ~3–10 seconds (JVM) | Native binary; cold start in tens of milliseconds |
| No competing SysML v2 validator emits SARIF | SARIF 2.1.0, JUnit XML, GCC-style plain text, JSON, and human text |
| Suppressed findings often vanish silently | Inline `// sysml-validate: disable=...` directives surface as SARIF `suppressions[]` audit records |
| Onboarding to a large existing model means fixing every legacy finding first | `--baseline` mode: record current findings, gate only new ones |
| Agentic clients can't write correct suppression syntax from training data alone | A bundled [agentskills.io](https://agentskills.io)-spec agent skill teaches Claude Code, Cursor, Copilot, Gemini CLI, OpenCode, Goose, Amp, Roo Code, and ~25 other clients how to invoke the validator and write suppressions without hallucinating syntax |
| Air-gapped deployments need self-contained artifacts | The OMG SysML v2 standard library (release `2026-04`, EPL-2.0) is embedded in the binary at compile time. No network. No submodule download at runtime. |

## Install

Download the release archive for your platform from the
[**Releases page**](https://github.com/coldboot-studio/sysml_cli/releases/latest)
and extract. Pre-built binaries are published for:

| Target | Archive |
|---|---|
| Linux x86_64 | `sysml-validate-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 | `sysml-validate-<version>-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Apple Silicon | `sysml-validate-<version>-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `sysml-validate-<version>-x86_64-pc-windows-msvc.zip` |

```bash
# Linux / macOS
tar xzf sysml-validate-<version>-<target>.tar.gz
./sysml-validate-<version>-<target>/bin/sysml-validate --version
```

```powershell
# Windows
Expand-Archive sysml-validate-<version>-x86_64-pc-windows-msvc.zip
.\sysml-validate-<version>-x86_64-pc-windows-msvc\bin\sysml-validate.exe --version
```

Each archive is a **verifiable bundle**: the binary plus its
SHA-256, Sigstore (cosign) keyless signature with Rekor inclusion,
detached GPG signature, CycloneDX 1.6 + SPDX 3.0 SBOMs, SLSA v1.0
Build Level 3 in-toto provenance, the full compliance pack, and the
agent skill. A `BUNDLE-MANIFEST.txt` at the bundle root lists every
file with its SHA-256 for one-command tree verification. The full
verification recipe is in [`docs/SECURITY.md`](docs/SECURITY.md).

## Quick start

```bash
sysml-validate validate model.sysml
sysml-validate validate src/                                     # walk a tree
sysml-validate validate src/ --format sarif > findings.sarif     # GitHub / GitLab / SonarQube
sysml-validate validate src/ --format junit                      # Jenkins / GitLab
sysml-validate validate src/ --format plain                      # screen readers, IDE problem matchers
sysml-validate validate src/ --baseline baseline.sarif           # only new findings fail
sysml-validate validate src/ --backend official \
  --official-command "sysml-validator --strict {file}"           # delegate to OMG Pilot
sysml-validate lsp                                               # LSP 3.x over stdio for VS Code / Neovim / Helix / Emacs
```

Full reference, all flags, configuration file format, rule catalog,
and per-platform CI recipes are in
[`docs/TECH_MANUAL.md`](docs/TECH_MANUAL.md).

## Build from source

For contributors and adopters that need an unsigned local build.
Production deployments should use the signed release archive above.

```bash
git clone --recurse-submodules https://github.com/coldboot-studio/sysml_cli
cd sysml_cli
cargo build --release
# binary at target/release/sysml-validate
```

Rust 1.85+ required (pinned in `rust-toolchain.toml`). The build
embeds the OMG SysML v2 standard library from the vendored submodule
at `vendor/sysml-v2-release/` (EPL-2.0; see [`NOTICE.md`](NOTICE.md)).
If you cloned without `--recurse-submodules`, run
`git submodule update --init --recursive` once before building.

## Documentation

| Document | Audience |
|---|---|
| [`docs/EXECUTIVE_SUMMARY.md`](docs/EXECUTIVE_SUMMARY.md) | Flag / SES decision-makers. One page. |
| [`docs/TECH_MANUAL.md`](docs/TECH_MANUAL.md) | Operators, CI engineers, model authors. Every flag, every rule, every recipe. |
| [`docs/SECURITY.md`](docs/SECURITY.md) | Verification recipes, vulnerability disclosure, NIST 800-53 cross-references. |
| [`docs/compliance/INDEX.md`](docs/compliance/INDEX.md) | Government reviewer entry point — maps reviewer role (SCRM / ATO / airworthiness / NASA / CMMC L2 / Section 508) to the document that answers their questions. |
| [`docs/AI_DEVELOPMENT_DISCLOSURE.md`](docs/AI_DEVELOPMENT_DISCLOSURE.md) | AI-assisted development disclosure with NIST AI RMF + SSDF-ML alignment. |
| [`skills/sysml-validate/SKILL.md`](skills/sysml-validate/SKILL.md) | The bundled agent skill — drop into any agentskills.io-compatible client. |

## Version history

Per-release notes, signed artifacts, SBOMs, and provenance for every
tagged release are on the
[Releases page](https://github.com/coldboot-studio/sysml_cli/releases).
The `git log` shows commit-level history.

## License

MIT — see [`LICENSE`](LICENSE). Embedded SysML v2 standard library
redistributed under EPL-2.0; full third-party attribution in
[`NOTICE.md`](NOTICE.md).
