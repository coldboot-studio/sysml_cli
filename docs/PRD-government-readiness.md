# PRD: Government-Readiness Upgrade of `sysml-validate`

| Field | Value |
|---|---|
| Status | Draft, Phase 0 complete |
| Owner | sysml-cli maintainers |
| Last updated | 2026-05-18 |
| Target audience | Maintainers; defense-prime evaluators; federal program offices |
| Related | [README.md](../README.md), [Cargo.toml](../Cargo.toml) |

---

## 1. Introduction / Overview

`sysml-validate` is a Rust CLI for validating SysML v2 and KerML textual
models. Today (v0.2.0) it is a **lexical preflight**: tokenizer, balanced-
delimiter check, statement-shape recognizer, duplicate-name-in-scope detector,
optional unresolved-reference warning, plus an `--official-command` escape
hatch that hands files to an external reference validator.

That posture is sound as a CI gate but not sufficient for two distinct
consumer needs that this PRD addresses:

1. **Trustworthy findings.** OMG SysML v2 and KerML are now *formal*
   specifications (`formal/26-03-02` and `formal/26-03-01`, September 2025;
   editorially revised March 2026 for ISO Fast-Track). Real conformance
   requires loading the standard library, resolving qualified names across
   files, enforcing several hundred normative OCL well-formedness constraints,
   and integrating with the Pilot Implementation as a reference oracle.
2. **Acceptable artifact.** US Government adopters — DoD programs, defense
   primes, federal civilian, NASA — require specific supply-chain and
   security artifacts (SSDF mapping, SBOM, SLSA provenance, signed releases,
   SARIF output, optional DO-330 TQL-5 qualification kit) before a tool can
   move through procurement and Authority-to-Operate review.

This PRD scopes the work in four phases. Phase 0 (consolidation +
restructuring) is complete in v0.2.0. Phase 1 (hygiene + output) is the next
deliverable.

## 2. Goals

- **G1.** Establish a single Rust source of truth. (Phase 0 — done.)
- **G2.** Produce machine-readable diagnostic output that drops directly into
  US Government DevSecOps pipelines (Platform One, Iron Bank, GitHub
  Advanced Security in GovCloud, GitLab Ultimate Federal).
- **G3.** Produce supply-chain artifacts that align with the post-M-26-05
  risk-based assurance posture: SBOM (CycloneDX + SPDX), SLSA Build L3
  provenance, signed releases.
- **G4.** Become a *real* SysML v2 validator: load the standard library,
  resolve names across files, port the OMG Pilot's named validator rules,
  and treat the Pilot as a differential-testing oracle.
- **G5.** Stay **offline-capable** and **fully self-contained** so the tool
  is deployable in DoD IL4/IL5 enclaves without policy exception.
- **G6.** Maintain a one-implementation, one-binary stance — no parallel
  language ports. Python distribution, when added, will wrap the Rust binary
  via maturin/PyO3.

## 3. Scope

### 3a. What's done (Phase 0, v0.2.0)

- Python re-implementation deleted; Rust is the source of truth.
- `src/main.rs` decomposed into modules: `lex.rs`, `diag.rs`, `validate.rs`,
  `backend.rs`, `report.rs`, `info.rs`.
- `--official-command` no longer spawns a shell. Argument template is parsed
  with shell-style quoting and invoked via positional argv.
- `--timeout <seconds>` flag added (parsed; not yet enforced — see open
  questions).
- Per-run metadata block added to text and JSON output: tool name and
  version, rule catalog version, RFC 3339 UTC timestamp, backend identity,
  ruleset flags.
- Rule catalog versioning introduced (`RULE_CATALOG_VERSION` in
  `src/report.rs`).
- Crate renamed from `sysml-validate-rs` to `sysml-validate`. Version 0.2.0.
- 15 unit tests pass on Windows.

### 3b. What's in scope (Phases 1-4)

- SARIF 2.1.0 and JUnit XML output formats with stable rule IDs.
- Suppression mechanism (`// sysml-validate: disable=<RULE>`) and
  `sysml-validate.toml` rule-severity config.
- Baseline/diff mode for incremental CI adoption.
- Cross-file name resolution, library loading, project manifest support
  (adopt Sensmetry's Sysand `.project.json` / `.meta.json` and `.kpar`).
- Port the OMG Pilot's named validator rules; differential test corpus.
- Thin LSP server.
- Government acceptance package: SSDF mapping, SBOM, SLSA L3 provenance,
  Sigstore + GPG signing, reproducible builds, NIST 800-53 control mapping,
  CMMC L2 deployment guide, optional DO-330 TQL-5 qualification kit, NASA
  NPR 7150.2D tool validation report, VPAT 2.5.

### 3c. Non-Goals (out of scope)

- **NG1.** A parallel Python implementation of the validator. Python
  distribution, if delivered, will be a maturin wheel around the Rust binary.
- **NG2.** A GUI client. The product is a CLI plus an LSP server. Editor
  vendors and SysON deliver the GUI story.
- **NG3.** A SysML v2 modeling editor or graphical renderer. Visualization
  may be added as an export (PlantUML / Graphviz), but not as an editor.
- **NG4.** A reimplementation of the Pilot's full KerML expression
  evaluator. Deep semantic checks (constraint evaluation, model-level
  expression evaluation) will be delegated to the Pilot via the hardened
  `--backend official` until a Rust evaluator is independently justified.
- **NG5.** A Systems Modeling API server. Client integration with a
  Pilot-hosted server is in scope as a Phase 4 differentiator; serving
  models is not.
- **NG6.** v1→v2 transformation (SysML 2.0 Part 4). Out of scope for this
  tool; users should use the Cameo / CATIA Magic 2026x path or the Pilot.
- **NG7.** Any feature that requires runtime network access by default.
  Network-touching subcommands (e.g., a future `sysand fetch`) must be
  separately gated and explicitly opt-in.

## 4. User Stories

Stories are grouped by phase. Each lists acceptance criteria. Stories marked
**[EPIC]** are larger than one focused session and will be broken down at
implementation time.

### Phase 1 — Hygiene + output (~ 2-4 weeks)

#### US-101: Emit SARIF 2.1.0 output

**Description.** As a DoD DevSecOps engineer, I want `sysml-validate
--format sarif` to emit SARIF 2.1.0 so that findings drop directly into
Iron Bank / GitHub Advanced Security / SonarQube / Azure DevOps.

**Acceptance Criteria:**
- [ ] `--format sarif` produces a SARIF 2.1.0 log validating against the
      OASIS schema.
- [ ] `runs[0].tool.driver` contains `name`, `version`, `semanticVersion`,
      `informationUri`, and a populated `rules` array (one entry per
      `SYSML*` code with `id`, `name`, `shortDescription`, `fullDescription`,
      `defaultConfiguration.level`, and `helpUri`).
- [ ] `runs[0].invocations[0]` records the command-line, start/end time
      (RFC 3339), exit code, and working directory.
- [ ] Each `result` has `ruleId`, `level`, `message.text`,
      `locations[0].physicalLocation.{artifactLocation,region.{startLine,startColumn}}`,
      and a stable `partialFingerprints.diagnosticHash/v1` value.
- [ ] `cargo test` passes including a schema-validation test of a fixture
      SARIF output.
- [ ] `--ci` short flag defaults to `--format sarif`.

#### US-102: Emit JUnit XML output

**Description.** As a Jenkins/GitLab CI user, I want `--format junit` to
produce JUnit-style XML so my pipeline's existing test reporters can
surface SysML findings.

**Acceptance Criteria:**
- [ ] `--format junit` writes a JUnit XML document grouping diagnostics
      into one `<testsuite>` per file.
- [ ] Errors produce `<failure>` nodes; warnings produce `<error>` only when
      `--fail-on-warning` is set, otherwise `<system-out>` annotation.
- [ ] Output validates against the Maven Surefire JUnit XSD.
- [ ] Fixture test covers both error and warning paths.

#### US-103: `--fail-on-warning` flag

**Description.** As a release engineer, I want `--fail-on-warning` so that
strict pipelines can gate on warnings without an external grep.

**Acceptance Criteria:**
- [ ] When set, exit code 1 if any warning is produced, regardless of error
      count.
- [ ] Help text documents the flag.
- [ ] Test covers the warning-only path returning exit 1.

#### US-104: Diagnostic suppression comments

**Description.** As a SysML model author, I want to suppress a specific
diagnostic on a specific line so I can incrementally adopt the validator
without forking my model.

**Acceptance Criteria:**
- [ ] A line comment `// sysml-validate: disable=SYSML041` on the same line
      as a diagnostic suppresses that diagnostic.
- [ ] A line comment `// sysml-validate: disable-next-line=SYSML041`
      suppresses on the next non-blank line.
- [ ] `// sysml-validate: disable=all` suppresses all rules on the line.
- [ ] Suppressed diagnostics are recorded in SARIF as `suppressions[]`
      entries with `kind: "inSource"`, not silently dropped.
- [ ] An info-level diagnostic (`SYSML050`, "unused suppression") fires
      when a suppression directive does not match any diagnostic.

#### US-105: `sysml-validate.toml` configuration file

**Description.** As a project lead, I want to commit a config file that
configures rule severity and project paths so my team gets identical results
without command-line incantations.

**Acceptance Criteria:**
- [ ] If `sysml-validate.toml` exists in the working directory or any
      ancestor, it is loaded automatically.
- [ ] Configurable: per-rule severity override (`error` / `warning` / `off`),
      file include/exclude globs, default `--format`, default `--strict`,
      project root.
- [ ] Command-line flags override file values.
- [ ] `--config <path>` loads an explicit file.
- [ ] `--no-config` skips file discovery.
- [ ] Loaded config path appears in the metadata block.

#### US-106: Baseline / diff mode

**Description.** As a team adopting the validator on a large model, I want
to record a baseline of existing findings so CI only fails on *new* findings.

**Acceptance Criteria:**
- [ ] `--baseline <file>` accepts a previous SARIF log.
- [ ] Findings whose `partialFingerprints.diagnosticHash/v1` matches an
      entry in the baseline are reported with `baselineState: "unchanged"`
      and do not affect the exit code.
- [ ] New findings have `baselineState: "new"` and *do* affect the exit
      code.
- [ ] A `--update-baseline` flag overwrites the baseline with the current
      run's output.

#### US-107: Stable diagnostic fingerprints

**Description.** As a tooling integrator, I want a stable fingerprint per
diagnostic so I can deduplicate across runs without using line numbers.

**Acceptance Criteria:**
- [ ] Each diagnostic exposes a `diagnosticHash/v1` derived from
      `(ruleId, file path normalized to project root, surrounding token
      context, suppression-trimmed message template)`.
- [ ] Two runs of the same input produce identical fingerprints.
- [ ] Changing an unrelated line does not change a diagnostic's fingerprint.

#### US-108: Enforce `--timeout` on official backend

**Description.** As a CI operator, I want the official backend to be
killed if it exceeds `--timeout` so a hung child cannot stall my pipeline.

**Acceptance Criteria:**
- [ ] When the official backend exceeds `--timeout` seconds, the child
      process is terminated (SIGKILL on Unix, `TerminateProcess` on
      Windows) and a `SYSML904` "timeout" diagnostic is emitted.
- [ ] Tested with a deliberately slow shell stub.
- [ ] Default timeout is 60 s (unchanged).

#### US-109: Reproducible-build setup [EPIC]

**Description.** As a defense prime SCRM reviewer, I want bit-identical
release binaries from identical source so I can verify provenance
independently.

**Acceptance Criteria:**
- [ ] `rust-toolchain.toml` pins the Rust version.
- [ ] `Cargo.lock` is committed (already true; document it).
- [ ] Build script honors `SOURCE_DATE_EPOCH`.
- [ ] CI runs `diffoscope` between two independent builds and fails on diff.
- [ ] `docs/REPRODUCING.md` documents the verification recipe.

#### US-110: SBOM generation in CI

**Description.** As a federal-civilian procurement officer, I want
CycloneDX 1.6 and SPDX 3.0 SBOMs attached to every release so I can review
the supply chain before approval.

**Acceptance Criteria:**
- [ ] CI generates CycloneDX 1.6 JSON via `cargo-cyclonedx` and SPDX 3.0
      via `syft` on each tagged release.
- [ ] Both SBOMs are attached to the GitHub Release.
- [ ] SBOMs include the NTIA Minimum Elements plus CISA 2025 additions
      (component hash, license, tool name, generation context, software
      producer).

#### US-111: Signed releases (Sigstore + GPG)

**Description.** As an Iron Bank reviewer, I want signed release artifacts
with public-log provenance so I can verify origin in an air-gapped enclave.

**Acceptance Criteria:**
- [ ] CI signs each release artifact with cosign (keyless OIDC,
      Rekor-logged) using GitHub Actions OIDC identity.
- [ ] CI also signs each release artifact with a long-lived GPG key whose
      public key is mirrored at `keys.openpgp.org` and a project-controlled
      URL.
- [ ] `docs/SECURITY.md` documents verification commands for both methods.

#### US-112: SLSA v1.0 Build L3 provenance

**Description.** As a Platform One Big Bang adopter, I want SLSA v1.0
Build L3 in-toto attestations so my pipeline can verify build integrity.

**Acceptance Criteria:**
- [ ] CI uses `actions/attest-build-provenance` (or
      `slsa-framework/slsa-github-generator`) to emit in-toto SLSA v1.0
      provenance.
- [ ] Provenance is published as an OCI referrer on `ghcr.io` and as a
      file on the GitHub Release.
- [ ] Build runs on hardened, isolated GitHub-hosted runners.

#### US-113: `SECURITY.md`, `OFFLINE.md`, vulnerability disclosure

**Description.** As a CISO, I want a documented vulnerability disclosure
policy and offline-deployment statement so I can accept the tool into an
ATO boundary.

**Acceptance Criteria:**
- [ ] `docs/SECURITY.md` covers supported versions, reporting channel
      (private security advisory), SLA, and verification commands.
- [ ] `docs/OFFLINE.md` enumerates the network-touching surfaces (today:
      none) and states the air-gap-friendly contract.
- [ ] `docs/THREAT_MODEL.md` enumerates trust boundaries (user input
      files; `--official-command` argv; environment variables) and the
      mitigations.

#### US-114: Section 508 polish

**Description.** As a federal user using a screen reader, I want the
default CLI output to be readable without ANSI / color decoration.

**Acceptance Criteria:**
- [ ] `NO_COLOR=1` and `--format plain` produce ANSI-free output.
- [ ] No color-only signaling: severity is always carried by the literal
      `ERROR` / `WARNING` / `INFO` text in addition to any color.
- [ ] `docs/accessibility.md` includes a draft VPAT 2.5 (Rev 508).

### Phase 2 — Become a real validator (~ 2-4 months)

#### US-201: Real parser [EPIC]

**Description.** As a SysML v2 model author, I want the validator to parse
the textual grammar correctly so diagnostics report the actual language
structures, not a token-pattern approximation.

**Acceptance Criteria:**
- [ ] Replace the lex-based statement-shape recognizer with a real
      parser, either by adopting `tree-sitter-sysml` (nomograph) as a
      dependency or by hand-writing recursive descent against the normative
      `.kebnf`.
- [ ] Parser emits an AST with span information.
- [ ] Existing 15 rule tests continue to pass against the AST-based
      implementation.
- [ ] At least 90% of the OMG Pilot's `kerml/` and `sysml/` example
      directories parse without error.

#### US-202: Library loader [EPIC]

**Description.** As a SysML v2 model author, I want my model to validate
against `ISQ::Mass`, `SI::kg`, `Geometry::Point`, etc. so I can use the
standard library.

**Acceptance Criteria:**
- [ ] Vendor the SysML v2 standard library `.sysml` / `.kerml` files from
      the OMG Release repository, pinned by release tag.
- [ ] `sysml-validate library-info` lists loaded library packages,
      element counts, source release tag.
- [ ] A loaded library provides a search scope for qualified-name
      resolution.
- [ ] `--library-path <path>` augments the default library scope with
      user-provided packages.

#### US-203: Qualified-name resolution [EPIC]

**Description.** As a SysML v2 model author, I want `import`, `private
import`, `alias`, and `A::B::C` to resolve across files within a project.

**Acceptance Criteria:**
- [ ] Implements KerML's qualified-name resolution algorithm including
      recursive `::**` imports and visibility (`public`, `protected`,
      `private`).
- [ ] An unresolved reference produces a `SYSML200` "unresolved name"
      error with the searched-scope chain in the message.
- [ ] Differential test: the Pilot's `sysml/src/examples/` parses with
      the same unresolved-name set, within an agreed delta.

#### US-204: Project manifest support

**Description.** As a SysML v2 project lead, I want a `.project.json`
file (Sysand-compatible) at the project root so all my files are validated
together.

**Acceptance Criteria:**
- [ ] If a `.project.json` is present at or above the working directory,
      it defines the project root and dependency set for resolution.
- [ ] If a `.kpar` archive is referenced, it is read as a dependency.
- [ ] `--project-root <path>` overrides discovery.

#### US-205: Port the Pilot's named validator rules [EPIC]

**Description.** As a SysML v2 model author, I want the same semantic
checks the Pilot runs, with the same rule names, so my diagnostics are
portable.

**Acceptance Criteria:**
- [ ] Port the following rules from the Pilot's Xtext validator suite,
      keeping the names: `checkFeatureParameterRedefinition`,
      `validateRedefinitionDirectionConformance`,
      `checkActionUsageSubactionSpecialization`, plus type / multiplicity /
      port-flow-direction conformance.
- [ ] Each ported rule has a `SYSML2xx` code and a fixture model that
      triggers it.
- [ ] Differential test against the Pilot on the official example
      corpus: any disagreement is documented as either a known false
      positive (bug) or a deliberate divergence (justified in a comment).

#### US-206: Thin LSP server

**Description.** As a SysML v2 model author using VS Code or Neovim, I
want `sysml-validate lsp` to provide hover, go-to-definition, and
diagnostics.

**Acceptance Criteria:**
- [ ] `sysml-validate lsp` starts a Language Server Protocol server on
      stdin/stdout.
- [ ] Supports: `textDocument/didOpen`, `didChange`, `didClose`;
      `textDocument/hover`; `textDocument/definition`; `textDocument/
      diagnostic` and `publishDiagnostics`.
- [ ] Reuses the parser and resolver from US-201 / US-203.
- [ ] Smoke-tested against a VS Code reference extension.

#### US-207: Differential test corpus

**Description.** As a maintainer, I want a CI job that compares
`sysml-validate`'s findings to the Pilot's on the public corpora named in
`corpus-info`.

**Acceptance Criteria:**
- [ ] CI job clones the corpus repos (network-allowed in CI only),
      validates with both implementations, and reports diffs.
- [ ] Diffs are categorized into: false positive, false negative, known
      divergence (with rationale).
- [ ] Pass criterion: zero unjustified diffs.

### Phase 3 — Government acceptance package (~ 1-3 months, mostly docs)

#### US-301: NIST SSDF mapping

**Description.** As a defense prime SSDF reviewer, I want a single
document that maps every NIST SP 800-218 practice (PO, PS, PW, RV) to a
piece of evidence in this repository.

**Acceptance Criteria:**
- [ ] `docs/compliance/ssdf-mapping.md` covers PO.1, PS.1-3, PW.4, PW.6,
      PW.7, RV.1-3 with links to specific repo artifacts (CI configs,
      branch protection settings, SECURITY.md, release process).
- [ ] A completed CISA Common Form (kept on file even though M-26-05
      made universal attestation risk-based) is referenced from this doc.

#### US-302: NIST 800-53 Rev 5 control mapping

**Description.** As an ATO reviewer, I want to see which NIST 800-53
controls this tool supports a system in satisfying.

**Acceptance Criteria:**
- [ ] `docs/compliance/nist-800-53-mapping.md` covers SA-11, SA-15,
      SR-3, SR-4, SR-11, SI-7, CM-7, AU-2, AU-12 with one-paragraph
      explanations of how `sysml-validate` participates in satisfying each.

#### US-303: CMMC L2 deployment guide

**Description.** As a CMMC L2 contractor, I want a deployment recipe
that does not undo my controls.

**Acceptance Criteria:**
- [ ] `docs/compliance/cmmc-l2-deployment.md` covers least-privilege
      execution, audit logging configuration, output retention guidance,
      and a hardened-container example.

#### US-304: DO-330 TQL-5 qualification kit skeleton

**Description.** As a DO-178C airworthiness program user, I want a
qualification kit so I can use `sysml-validate` to discharge a verification
objective.

**Acceptance Criteria:**
- [ ] `docs/compliance/do-330/` contains template documents for: Tool
      Operational Requirements (TOR), Tool Qualification Plan (TQP), Tool
      Quality Assurance Plan, Tool Configuration Management Plan, Tool
      Verification Cases and Procedures (TVCP), Tool Verification Cases
      and Results (TVCR), Tool Accomplishment Summary (TAS).
- [ ] Tool Criterion is set to Criterion 3 (verification tool, no
      replacement of activity) and TQL-5 is targeted.
- [ ] Filled-in TOR and TVCR for one representative ruleset; the rest
      are templates.

#### US-305: NASA NPR 7150.2D tool validation report

**Description.** As a NASA Class A/B/C program user, I want a tool
validation report aligned with NPR 7150.2D §4.4.8 / §4.5.6.

**Acceptance Criteria:**
- [ ] `docs/compliance/nasa-npr-7150-2d-tool-validation.md` documents
      the intended use, validation evidence, and limitations.

#### US-306: VPAT 2.5 (Rev 508)

**Description.** As a federal agency procurement officer, I want a
completed VPAT so I can include `sysml-validate` in an ICT procurement.

**Acceptance Criteria:**
- [ ] `docs/compliance/vpat-2.5-rev508.md` is published.

#### US-307: `--fips` build flag

**Description.** As a federal agency operator, I want a build that uses
only FIPS 140-3 validated cryptography for any signing or hashing.

**Acceptance Criteria:**
- [ ] `cargo build --features fips` selects a FIPS-validated module
      (AWS-LC-FIPS or RustCrypto FIPS variant) for any future signing
      operation.
- [ ] Today `--fips` is a no-op because the tool performs no cryptographic
      operations; the build is documented and reserved for Phase 4.

### Phase 4 — Differentiation (open-ended)

#### US-401: Systems Modeling API client

**Description.** As a DoDI 5000.97 program implementing an Authoritative
Source of Truth, I want `sysml-validate` to read models from a Systems
Modeling API server (the OMG REST/HTTP PSM).

**Acceptance Criteria:**
- [ ] `sysml-validate validate --api-url <url> --project <id>` fetches
      and validates a project via the OMG SysML v2 API.
- [ ] OpenAPI client is generated from the official spec (`formal/26-03-04`
      machine-readable bundle).
- [ ] API access is gated on `--allow-network` (not on by default).

#### US-402: KPAR import/export and JSON-AS I/O

**Description.** As a SysML v2 tool interoperator, I want to consume
`.kpar` archives and produce JSON-AS payloads.

**Acceptance Criteria:**
- [ ] `sysml-validate validate <project.kpar>` reads the archive and
      validates its contents.
- [ ] `sysml-validate export --format kpar` produces a `.kpar` archive
      from a validated project tree.
- [ ] JSON output conforms to the SysML v2 Abstract Syntax JSON Schema.

#### US-403: Requirements traceability matrix export

**Description.** As an MBSE program manager, I want a CSV / ReqIF /
Markdown matrix of requirement satisfactions and verifications for
program-office reporting.

**Acceptance Criteria:**
- [ ] `sysml-validate trace --format {csv,reqif,markdown}` emits a
      satisfaction/verification matrix.

#### US-404: Visualization export

**Description.** As a reviewer, I want PlantUML / Graphviz / Mermaid
exports of part decomposition and requirement satisfaction graphs.

**Acceptance Criteria:**
- [ ] `sysml-validate viz --kind {parts,requirements} --format {plantuml,
      graphviz,mermaid}` emits the diagram source.

#### US-405: Imandra / Z3 bridge for constraint counterexamples

**Description.** As a model author with unsatisfiable constraints, I
want a counterexample so I can fix them.

**Acceptance Criteria:**
- [ ] `sysml-validate validate --solver z3` invokes a Z3 / Imandra
      backend for constraint-evaluation diagnostics and surfaces
      counterexamples in the diagnostic message.

## 5. Functional Requirements

### Diagnostic and output

- **FR-1.** The tool MUST emit diagnostics in three output formats: `text`
  (default for humans), `json` (legacy), `sarif` (default in CI), and
  `junit` (Jenkins-compatible).
- **FR-2.** Every diagnostic MUST carry a stable rule ID from the `SYSML`
  namespace, a severity, a message, a file path, and (when available) a
  line/column range.
- **FR-3.** Every run MUST include a metadata block with: tool name and
  version, rule catalog version, RFC 3339 UTC timestamp, backend identity,
  ruleset flags, and the configuration file path if one was loaded.
- **FR-4.** Every diagnostic MUST expose a `diagnosticHash/v1` fingerprint
  stable across runs (see US-107).
- **FR-5.** Exit codes: `0` no errors, `1` errors found (or warnings if
  `--fail-on-warning`), `2` CLI / backend configuration error.

### Validation engine

- **FR-6.** The native backend MUST validate `.sysml` and `.kerml` files
  for: lexical correctness, balanced delimiters, statement shape, duplicate
  scope members (Phase 0).
- **FR-7.** Phase 2: the native backend MUST resolve qualified names across
  files within a project, honor `import` / `private import` / `alias` /
  `::**`, and load the SysML v2 standard library.
- **FR-8.** Phase 2: the native backend MUST implement the named validator
  rules listed in US-205 with the Pilot's behavior.
- **FR-9.** The official backend (`--backend official`) MUST invoke the
  user-supplied command via positional argv. It MUST NOT spawn a shell.
- **FR-10.** The official backend MUST kill the child process if it
  exceeds `--timeout` (US-108).
- **FR-11.** A `--strict` flag MUST enable the unresolved-reference
  warning (`SYSML040`); behavior preserved.

### Configuration

- **FR-12.** A `sysml-validate.toml` file MAY configure per-rule severity,
  include/exclude globs, default `--format`, default `--strict`, and the
  project root (US-105).
- **FR-13.** Diagnostic suppression directives in source comments MUST be
  honored (US-104).

### Supply chain and release

- **FR-14.** Every tagged release MUST publish CycloneDX 1.6 and SPDX 3.0
  SBOMs (US-110).
- **FR-15.** Every tagged release MUST publish SLSA v1.0 Build L3
  in-toto provenance (US-112).
- **FR-16.** Every tagged release MUST be signed with both Sigstore
  keyless OIDC (Rekor-logged) and a long-lived GPG key (US-111).
- **FR-17.** Source-to-binary builds MUST be reproducible byte-for-byte
  given the same source revision and toolchain (US-109).

### Security and posture

- **FR-18.** The `validate` subcommand MUST NOT make network calls.
- **FR-19.** The tool MUST NOT auto-update.
- **FR-20.** The tool MUST NOT emit telemetry.
- **FR-21.** The tool MUST run on hardened, FIPS-mode-capable Linux
  (RHEL/UBI) and Windows hosts.

## 6. Design / Technical Considerations

### Architecture

- **D1. Differential testing against the Pilot is the strategy, not a
  full reimplementation.** The Pilot's Java/Xtext stack remains the
  reference oracle. Where `sysml-validate` disagrees, the Pilot wins
  unless the disagreement is documented and justified.
- **D2. Adopt Sysand's project format and `.kpar` archive standard.** Do
  not invent a competing manifest format.
- **D3. Use `tree-sitter-sysml` or `syster-base` rather than hand-writing
  a parser from scratch, unless an evaluation shows neither meets quality
  needs.** Hand-rolling is a multi-month commitment; bridging is days.
- **D4. License: re-license from MIT to Apache-2.0.** The patent grant is
  important for DoD adoption and is a one-time tax. (Open question — see
  §9.)
- **D5. The Rust binary is the source of truth. Python distribution, if
  added, is a maturin wheel that bundles the Rust binary.**

### Dependencies posture

- **D6.** Phase 0 maintained the zero-external-crate posture. Phase 1
  will introduce a small, well-vetted set: `sha2` (input/result hashing
  for fingerprints and provenance), a SARIF serializer (either hand-rolled
  or `serde_json` + `sarif-rs`), a TOML parser (`toml` + `serde`). Each
  dependency must have a stable upstream and a clear FIPS path where
  applicable.

### Performance

- **D7.** Lexical-only validation today is constant-factor fast. Phase 2's
  resolver should target sub-second validation on the OMG release
  examples directory.
- **D8.** The LSP server (US-206) must support incremental reparse;
  full-file reparse on every keystroke is not acceptable.

### Compatibility

- **D9.** The exit code contract (`0` / `1` / `2`) is stable and must not
  change.
- **D10.** Old `--format json` output continues to be emitted for
  backwards compatibility; `--format sarif` is the recommended CI default
  going forward.

## 7. Success Metrics

- **M1.** SARIF output is consumed without modification by GitHub Advanced
  Security, SonarQube, and Iron Bank pipelines.
- **M2.** The Pilot differential test (US-207) reports zero unjustified
  divergences on the official corpus.
- **M3.** A defense prime can complete a SCRM review of a release using
  only the published artifacts (SBOM, SLSA provenance, signatures,
  SSDF mapping) — no additional information requests.
- **M4.** A federal-civilian program can ingest the VPAT and proceed to
  ICT procurement without additional accessibility testing.
- **M5.** Build is bit-reproducible across two independent CI runs
  (`diffoscope` reports zero differences).
- **M6.** `sysml-validate validate` on the SysML v2 release `sysml/`
  examples directory finishes in under 5 seconds on a recent laptop
  (Phase 2 acceptance).

## 8. Open Questions

- **Q1. License re-license from MIT to Apache-2.0?** Trade-off: patent
  grant vs. one-time contributor outreach for relicense consent. Default
  position: yes, do it before Phase 1 ships.
- **Q2. Hand-rolled parser vs. `tree-sitter-sysml` vs. `syster-base`?**
  Default position: evaluate `tree-sitter-sysml` first; it gives
  incremental reparse for free and is the most actively maintained.
- **Q3. Should the tool ship a maturin/PyO3 wheel on PyPI?** Default
  position: yes in Phase 1, because Sysand and the SysML v2 API client
  are Python-first, and our Phase 0 consolidation freed the user-facing
  `sysml-validate` script name on PyPI.
- **Q4. SARIF rule URIs — where do they point?** Default position: a
  `/rules/<RULE-ID>.md` directory in this repository, with explanations
  and spec-section citations. Build out as rules are ported (Phase 2).
- **Q5. DO-330 qualification is expensive. Do we invest in the kit before
  a customer with airworthiness requirements asks?** Default position:
  skeleton only in Phase 3; fill in for a paying customer.
- **Q6. Should we adopt a CycloneDX-only SBOM and skip SPDX?** Both
  formats are commonly required; default position is both.
- **Q7. What's the policy on dependency version pinning vs. version
  ranges?** Default position: pinned exact versions in `Cargo.lock`,
  caret ranges in `Cargo.toml`. SBOM emits the pinned versions.

## 9. Appendix: Citations

Primary sources referenced in this PRD:

- OMG SysML v2 specifications:
  [About-KerML 1.0](https://www.omg.org/spec/KerML/1.0/About-KerML),
  [About-SysML 2.0](https://www.omg.org/spec/SysML/2.0/About-SysML),
  [Systems Modeling API & Services 1.0](https://www.omg.org/spec/SystemsModelingAPI),
  [OMG SysML page](https://www.omg.org/sysml/sysmlv2/).
- OMG Pilot Implementation:
  [github.com/Systems-Modeling/SysML-v2-Pilot-Implementation](https://github.com/Systems-Modeling/SysML-v2-Pilot-Implementation),
  [SysML-v2-Release](https://github.com/Systems-Modeling/SysML-v2-Release),
  [SysML-v2-API-Services](https://github.com/Systems-Modeling/SysML-v2-API-Services).
- Sensmetry tooling and project format: [Syside](https://sensmetry.com/syside/),
  [sysml-2ls](https://github.com/sensmetry/sysml-2ls),
  [Sysand](https://github.com/sensmetry/sysand).
- OMB and policy: [M-26-05](https://www.whitehouse.gov/wp-content/uploads/2026/01/M-26-05-Adopting-a-Risk-based-Approach-to-Software-and-Hardware-Security.pdf),
  [CISA SSDF Common Form](https://www.cisa.gov/sites/default/files/2024-04/Self_Attestation_Common_Form_FINAL_508c.pdf),
  [DoDI 5000.97 Digital Engineering](https://www.esd.whs.mil/Portals/54/Documents/DD/issuances/dodi/500097p.PDF),
  [DoD Software Development and OSS Memo](https://dodcio.defense.gov/Portals/0/Documents/Library/SoftwareDev-OpenSource.pdf).
- NIST and standards: [SP 800-218 SSDF](https://csrc.nist.gov/Projects/ssdf),
  [SP 800-53 Rev 5](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final),
  [FIPS 140-3](https://csrc.nist.gov/pubs/fips/140-3/final),
  [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html),
  [SLSA v1.0](https://slsa.dev/spec/v1.0/levels),
  [NTIA SBOM Minimum Elements](https://www.ntia.gov/sites/default/files/publications/sbom_minimum_elements_report_0.pdf),
  [DoD Cloud SRG](https://public.cyber.mil/dccs),
  [Access Board ICT](https://www.access-board.gov/ict).
- Safety: [RTCA DO-330](https://www.rtca.org),
  [NASA NPR 7150.2D](https://nodis3.gsfc.nasa.gov/displayDir.cfm?t=NPR&c=7150&s=2D).
