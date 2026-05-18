# Executive Summary: `sysml-validate`

| Field | Value |
|---|---|
| Status | v0.4.0 (Phase 0 + Phase 1 Batches A/B/C complete) |
| Stage | Preflight validator; on a roadmap to government-acceptance-grade conformance tooling |
| Last updated | 2026-05-18 |
| Related | [README.md](../README.md), [PRD-government-readiness.md](PRD-government-readiness.md) |

## What it is

`sysml-validate` is a single-binary Rust CLI that validates SysML v2 and
KerML textual models against the OMG public release layout
(`Systems-Modeling/SysML-v2-Release`). It is built to run as a fast
preflight gate in CI/CD pipelines — including air-gapped DoD enclaves —
without network access, telemetry, or auto-update.

The tool is intentionally conservative: it catches deterministic textual
issues quickly and cheaply, and delegates full language conformance to
the OMG Pilot Implementation via a hardened `--official-command`
escape hatch. The Pilot remains the reference oracle; `sysml-validate`'s
job is to fail fast on lexical and structural defects before the
heavyweight backend is invoked.

## What it does today (v0.4.0)

### Validation engine (native backend)

- Lexes `.sysml` and `.kerml` files; checks balanced braces / parentheses
  / brackets, unterminated strings and block comments, invalid control
  characters, and declaration terminators.
- Recognizes the shape of `package`, `import`, `alias`, `dependency`,
  and definition / usage statements.
- Detects duplicate member names within the same lexical scope.
- Optional unresolved qualified-name warning under `--strict`.
- Twenty-one diagnostic codes in the `SYSML0xx` namespace, each with
  stable severity, rule metadata, and a `helpUri`.

### Official-backend delegation

- `--backend official --official-command "<template>"` hands files to an
  external reference validator (e.g., the OMG Pilot). The template is
  shell-tokenized and invoked via positional argv — **no shell process
  is spawned**, so metacharacters cannot inject.
- `--timeout <seconds>` enforces a hard kill on hung children
  (`SYSML904`); stdout/stderr are drained in worker threads to avoid
  pipe deadlocks.

### CI/CD output formats

- `--format sarif` emits **SARIF 2.1.0** (the lingua franca for GitHub
  Advanced Security, GitLab Ultimate, Iron Bank, SonarQube, Azure DevOps)
  with a fully populated `tool.driver.rules` array, invocation block,
  per-result `partialFingerprints.diagnosticHash/v1`, and
  `suppressions[]` entries for inline-suppressed findings.
- `--format junit` emits Maven Surefire-compatible XML for Jenkins /
  GitLab pipelines.
- `--format json` is the legacy structured format; `text` is the human
  default. Every output includes a metadata block (tool + version, rule
  catalog version, RFC 3339 timestamp, backend identity, config path).

### Adoption and incremental rollout

- **Inline suppressions** via `// sysml-validate: disable=<RULE>`,
  `disable-next-line`, and `disable=all`. Dead directives surface as
  `SYSML050`; invalid syntax as `SYSML060`. Suppressed findings stay in
  the result list and ride to SARIF as `kind: "inSource"` audit records.
- **Baseline / diff mode** (`--baseline <file>` + `--update-baseline`):
  match findings by `(ruleId, diagnosticHash/v1)` so teams can adopt the
  validator on a large existing model without fixing every legacy
  finding up front. New findings fail CI; unchanged do not.
- **`sysml-validate.toml`** auto-discovered in any ancestor directory:
  per-rule severity overrides, include/exclude globs (hand-written
  100-line matcher, no `globset` dep), default format/strict/fail-on-
  warning, project root. `--config` and `--no-config` are explicit
  overrides. Unknown TOML fields are rejected so typos surface early.
- `--fail-on-warning` for strict CI gates.

### Security posture

- No network access in any subcommand.
- No automatic updates, no telemetry.
- `--official-command` is argv-invoked, never shell-evaluated.
- Designed for air-gapped DoD IL4 / IL5 deployment contexts.

### Dependency budget

`sha2`, `wait-timeout`, `serde` + `serde_json`, `toml`. The SARIF
emitter, JUnit emitter, and glob matcher are hand-written and
dependency-free.

## What's planned

The roadmap is captured in [PRD-government-readiness.md](PRD-government-readiness.md).
The thesis is that two distinct consumer needs — **trustworthy
findings** and **acceptable supply-chain artifacts** — drive a four-
phase plan.

### Phase 1 — Hygiene and supply-chain artifacts (remainder of the phase)

Output and configuration work is done (US-101 through US-108). The
remaining Phase 1 work targets government acceptance fundamentals:

- **US-109** Reproducible bit-identical release builds (`SOURCE_DATE_EPOCH`,
  pinned toolchain, `diffoscope` in CI).
- **US-110** CycloneDX 1.6 + SPDX 3.0 SBOM generation on every tagged
  release, with NTIA + CISA 2025 minimum elements.
- **US-111** Signed releases via Sigstore (keyless OIDC, Rekor-logged)
  **and** a long-lived GPG key.
- **US-112** SLSA v1.0 Build L3 in-toto provenance published as OCI
  referrer and on the GitHub Release.
- **US-113** `SECURITY.md`, `OFFLINE.md`, `THREAT_MODEL.md`, and a
  documented vulnerability disclosure channel.
- **US-114** Section 508 polish: `NO_COLOR=1`, `--format plain`, no
  color-only severity signaling, draft VPAT 2.5.

### Phase 2 — Become a real validator (~2-4 months)

- **US-201** Real parser. Replace the lex-based statement-shape recognizer
  with either `tree-sitter-sysml` or hand-written recursive descent
  against the normative `.kebnf`. AST with span info.
- **US-202** Standard-library loader (vendored from the OMG Release,
  pinned by tag) so `ISQ::Mass`, `SI::kg`, `Geometry::Point`, etc.
  resolve.
- **US-203** Qualified-name resolution across files, with `import`,
  `private import`, `alias`, recursive `::**`, and visibility honored.
- **US-204** Sysand-compatible project manifest (`.project.json` /
  `.meta.json` / `.kpar`).
- **US-205** Port the OMG Pilot's named validator rules
  (`checkFeatureParameterRedefinition`, `validateRedefinitionDirectionConformance`,
  `checkActionUsageSubactionSpecialization`, type / multiplicity /
  port-flow-direction conformance) under `SYSML2xx` codes.
- **US-206** Thin LSP server (`sysml-validate lsp`) for VS Code / Neovim
  with hover, go-to-definition, diagnostics.
- **US-207** Differential test corpus: CI compares findings against the
  Pilot on public corpora; zero unjustified diffs is the pass criterion.

### Phase 3 — Government acceptance package (~1-3 months, mostly docs)

- **US-301** NIST SP 800-218 SSDF mapping (PO, PS, PW, RV) with linked
  repo evidence and a completed CISA Common Form on file.
- **US-302** NIST SP 800-53 Rev 5 control mapping (SA-11, SA-15, SR-3,
  SR-4, SR-11, SI-7, CM-7, AU-2, AU-12).
- **US-303** CMMC L2 deployment guide.
- **US-304** DO-330 TQL-5 qualification kit skeleton (TOR, TQP, TVCP,
  TVCR, TAS templates) for DO-178C airworthiness users.
- **US-305** NASA NPR 7150.2D §4.4.8 / §4.5.6 tool validation report.
- **US-306** Completed VPAT 2.5 (Rev 508) for ICT procurement.
- **US-307** `--fips` build flag selecting FIPS 140-3 validated crypto
  (AWS-LC-FIPS or RustCrypto FIPS variant) for future signing operations.

### Phase 4 — Differentiation (open-ended)

- **US-401** Systems Modeling API client (`--api-url`, `--project`)
  against the OMG REST/HTTP PSM; network-gated.
- **US-402** `.kpar` import/export and JSON-AS payloads conforming to
  the SysML v2 Abstract Syntax JSON Schema.
- **US-403** Requirements traceability matrix export
  (`trace --format {csv,reqif,markdown}`).
- **US-404** Visualization export
  (`viz --kind {parts,requirements} --format {plantuml,graphviz,mermaid}`).
- **US-405** Imandra / Z3 bridge surfacing constraint counterexamples.

## Strategic posture

- **G1** One Rust source of truth; no parallel Python implementation
  (any Python distribution will be a maturin wheel around the Rust
  binary).
- **G2** Output drops directly into US Government DevSecOps pipelines
  (Platform One, Iron Bank, GitHub Advanced Security GovCloud, GitLab
  Ultimate Federal).
- **G3** Supply-chain artifacts align with the post-M-26-05 risk-based
  assurance posture: SBOM, SLSA L3, signed releases.
- **G4** Differential testing against the Pilot is the conformance
  strategy, not full reimplementation. The Pilot wins disagreements
  unless a divergence is documented and justified.
- **G5** Offline-capable and self-contained, deployable in IL4/IL5
  enclaves without policy exception.
- **G6** One implementation, one binary. No GUI, no v1→v2 transform,
  no API server, no full KerML expression evaluator (delegated to the
  Pilot until independently justified).

## Success metrics

- SARIF accepted unmodified by GitHub Advanced Security, SonarQube, and
  Iron Bank pipelines.
- Zero unjustified divergences from the Pilot on the official corpus.
- A defense prime can complete a SCRM review of a release using only
  the published artifacts.
- Federal-civilian programs can ingest the VPAT and proceed to ICT
  procurement without additional accessibility testing.
- Bit-reproducible builds across independent CI runs (`diffoscope`
  reports zero differences).
- `validate` on the SysML v2 release `sysml/` examples directory
  finishes in under 5 seconds on a recent laptop (Phase 2 acceptance).
