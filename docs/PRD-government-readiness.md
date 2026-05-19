# PRD: Government-Readiness Upgrade of `sysml-validate`

| Field | Value |
|---|---|
| Status | Draft. **Phase 0 + Phase 1 (A/B/C/D/E) + Phase 2 Batches F (US-202), G (US-203), G.5 (typed-usage `:`), H (US-205 scoped structural rules)** complete (v0.7.0). Phase 2 continues with US-204 (project manifest), US-205 deeper rules requiring AST, and US-207 (differential test). |
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

#### US-101: Emit SARIF 2.1.0 output — **DONE (v0.3.0)**

**Description.** As a DoD DevSecOps engineer, I want `sysml-validate
--format sarif` to emit SARIF 2.1.0 so that findings drop directly into
Iron Bank / GitHub Advanced Security / SonarQube / Azure DevOps.

**Acceptance Criteria:**
- [x] `--format sarif` produces a SARIF 2.1.0 log. Implemented in
      [sarif.rs](../src/sarif.rs).
- [x] `runs[0].tool.driver` contains `name`, `version`, `semanticVersion`,
      `informationUri`, and a populated `rules` array (one entry per
      `SYSML*` code with `id`, `name`, `shortDescription`, `fullDescription`,
      `defaultConfiguration.level`, and `helpUri`). The catalog is the
      single source of truth in [rules.rs](../src/rules.rs).
- [x] `runs[0].invocations[0]` records the command-line, start/end time
      (RFC 3339), exit code, working directory, and `executionSuccessful`.
- [x] Each `result` has `ruleId`, `level`, `message.text`,
      `locations[0].physicalLocation.{artifactLocation,region}` and a stable
      `partialFingerprints.diagnosticHash/v1` value.
- [x] `cargo test` includes a parse-back test plus a fingerprint test plus
      Windows-path → file:// URI test.
- [x] `--ci` short flag defaults to `--format sarif`.
- [ ] **Deferred to Batch D:** strict-schema validation against the
      OASIS JSON Schema (requires a JSON Schema crate); manual structure
      tests pass today; baseline mode and IDE consumers validate the
      shape in practice.

#### US-102: Emit JUnit XML output — **DONE (v0.4.0)**

**Description.** As a Jenkins/GitLab CI user, I want `--format junit` to
produce JUnit-style XML so my pipeline's existing test reporters can
surface SysML findings.

**Acceptance Criteria:**
- [x] `--format junit` writes a JUnit XML document grouping diagnostics
      into one `<testsuite>` per file. Implemented in
      [junit.rs](../src/junit.rs); hand-rolled emitter, no XML crate
      dependency.
- [x] Errors produce `<failure>` nodes; warnings produce `<error>` only
      when `--fail-on-warning` is set, otherwise `<system-out>` annotation.
- [x] Suppressed diagnostics are excluded from the testcase count.
- [x] XML special characters and quotes are escaped in attributes and
      text content.
- [x] Fixture tests cover error / warning / suppressed / escape paths.
- [ ] **Deferred:** strict XSD validation against Maven Surefire schema
      (no XSD validator in the dependency budget today; manual structure
      tests pass and the output is accepted by Jenkins JUnit reporters
      that practice "be liberal in what you accept").

#### US-103: `--fail-on-warning` flag — **DONE (v0.2.1)**

**Description.** As a release engineer, I want `--fail-on-warning` so that
strict pipelines can gate on warnings without an external grep.

**Acceptance Criteria:**
- [x] When set, exit code 1 if any warning is produced, regardless of error
      count. Implemented in [main.rs](../src/main.rs).
- [x] Help text documents the flag.
- [x] End-to-end smoke test (`--strict --fail-on-warning` on a model with
      a `SYSML040` warning) returns exit 1.

#### US-104: Diagnostic suppression comments — **DONE (v0.3.0)**

**Description.** As a SysML model author, I want to suppress a specific
diagnostic on a specific line so I can incrementally adopt the validator
without forking my model.

**Acceptance Criteria:**
- [x] `// sysml-validate: disable=SYSML041` on the same line as a
      diagnostic suppresses that diagnostic. Implemented in
      [suppress.rs](../src/suppress.rs) + [lex.rs](../src/lex.rs).
- [x] `// sysml-validate: disable-next-line=SYSML041` suppresses on the
      next non-blank line (the scanner tracks non-blank lines via a
      BTreeSet during scanning).
- [x] `// sysml-validate: disable=all` suppresses all rules on the line.
- [x] Comma-separated rule lists are supported
      (`disable=SYSML041,SYSML040`).
- [x] A warning-level `SYSML050` "unused suppression" fires when a
      directive doesn't match any diagnostic.
- [x] An invalid directive form produces `SYSML060`.
- [x] **Closed in Batch C (v0.4.0):** SARIF `suppressions[].kind =
      "inSource"` payloads. The suppression model was reworked from "drop"
      to "mark and keep": suppressed diagnostics stay on the result list,
      `Diagnostic::suppression: Option<String>` carries the justification,
      and the SARIF emitter renders them as `suppressions[]` with
      `kind: "inSource"` and `status: "accepted"`. Text and JSON output
      filter them by default; `--show-suppressed` reveals them.

#### US-105: `sysml-validate.toml` configuration file — **DONE (v0.3.0)**

**Description.** As a project lead, I want to commit a config file that
configures rule severity and project paths so my team gets identical
results without command-line incantations.

**Acceptance Criteria:**
- [x] If `sysml-validate.toml` exists in the working directory or any
      ancestor, it is loaded automatically. Implemented in
      [config.rs](../src/config.rs).
- [x] Configurable: per-rule severity override (`error` / `warning` /
      `info` / `off`), default `--format`, default `--strict`, default
      `--fail-on-warning`, `project_root`.
- [x] Command-line flags always override file values (the
      `cli_set_*` markers in [main.rs](../src/main.rs)).
- [x] `--config <path>` loads an explicit file.
- [x] `--no-config` skips file discovery.
- [x] Unknown TOML fields are rejected (`deny_unknown_fields`) so typos
      surface at validation time, not silently.
- [x] **Closed in Batch C (v0.4.0):** file include/exclude globs.
      Implemented with a hand-written 100-line matcher in
      [glob.rs](../src/glob.rs) — supports `*`, `**`, `?`, OS-independent
      path separators. No `globset` dep needed.
- [x] **Closed in Batch C (v0.4.0):** loaded config path and baseline
      path appear in the metadata block ([report.rs](../src/report.rs)
      `RunMetadata::config_path`, `baseline_path`).

#### US-106: Baseline / diff mode — **DONE (v0.4.0)**

**Description.** As a team adopting the validator on a large model, I
want to record a baseline of existing findings so CI only fails on *new*
findings.

**Acceptance Criteria:**
- [x] `--baseline <file>` accepts a previous SARIF log. Implemented in
      [baseline.rs](../src/baseline.rs).
- [x] Findings whose `(ruleId, partialFingerprints.diagnosticHash/v1)`
      matches an entry in the baseline are reported with
      `baselineState: "unchanged"` and do not affect the exit code.
- [x] New findings have `baselineState: "new"` and *do* affect the exit
      code. The exit-code computation walks all results and treats
      unchanged + suppressed as non-failing.
- [x] `--update-baseline` overwrites the baseline with the current run's
      SARIF and forces exit 0, so a single command both seeds and accepts
      the current state on a fresh project.
- [x] `--update-baseline` requires `--format sarif` (the baseline format)
      and `--baseline <path>`; the CLI parser enforces both.
- [x] End-to-end smoke test verified: run 1 seeds the baseline (exit 0),
      run 2 classifies the same finding as `unchanged` (exit 0), an
      injected new finding flips to `new` and exit 1.

#### US-107: Stable diagnostic fingerprints — **DONE (v0.2.1)**

**Description.** As a tooling integrator, I want a stable fingerprint per
diagnostic so I can deduplicate across runs without using line numbers.

**Acceptance Criteria:**
- [x] Each diagnostic exposes a fingerprint derived from `(rule code, file
      path normalized to project root, message template with literal
      identifiers and numbers genericized)`. Implemented in
      [diag.rs](../src/diag.rs).
- [x] Two runs of the same input produce identical fingerprints.
- [x] Changing an unrelated line does not change a diagnostic's
      fingerprint (position is excluded from the hash).
- [x] Fingerprint is normalized for Windows vs POSIX path separators.
- [ ] **Deferred to Phase 2 (AST-aware):** "surrounding token context"
      stability. Today, two distinct diagnostics from the same rule on the
      same file with identical genericized messages may collide; consumers
      should disambiguate by position when they care.

#### US-108: Enforce `--timeout` on official backend — **DONE (v0.2.1)**

**Description.** As a CI operator, I want the official backend to be
killed if it exceeds `--timeout` so a hung child cannot stall my pipeline.

**Acceptance Criteria:**
- [x] When the official backend exceeds `--timeout` seconds, the child
      process is terminated (`wait_timeout::ChildExt::wait_timeout` +
      `Child::kill`) and a `SYSML904` "timeout" diagnostic is emitted.
      Implemented in [backend.rs](../src/backend.rs).
- [x] `cargo test` includes a deliberately slow child (`Start-Sleep 30`
      on Windows, `sleep 30` elsewhere) that proves the timeout fires
      within the 15 s test budget.
- [x] Default timeout is 60 s (unchanged).
- [x] stdout / stderr are drained in worker threads so a chatty child
      cannot deadlock on a full pipe.

#### US-109: Reproducible-build setup [EPIC] — **DONE (v0.4.0), one item deferred**

**Description.** As a defense prime SCRM reviewer, I want bit-identical
release binaries from identical source so I can verify provenance
independently.

**Acceptance Criteria:**
- [x] [`rust-toolchain.toml`](../rust-toolchain.toml) pins Rust 1.85.0
      with explicit components.
- [x] `Cargo.lock` is committed (verified).
- [x] [`Cargo.toml`](../Cargo.toml) `[profile.release]` sets
      `codegen-units = 1`, `lto = "fat"`, `strip = "symbols"`,
      `panic = "abort"`, `incremental = false` — all required for
      determinism.
- [x] [`.cargo/config.toml`](../.cargo/config.toml) applies
      `--remap-path-prefix` for Linux, macOS, Windows runner paths so
      embedded debug info doesn't vary by host.
- [x] Release workflow reads `SOURCE_DATE_EPOCH` from the tag commit's
      author date.
- [x] [`docs/REPRODUCING.md`](REPRODUCING.md) documents the verification
      recipe including a `diffoscope` walkthrough for mismatch debugging.
- [ ] **Deferred to first real release:** a CI job that runs `diffoscope`
      between two independent fresh builds of the same tag and fails the
      release on diff. The recipe is documented; the job itself is a
      one-PR follow-up once a baseline release exists to compare against.

#### US-110: SBOM generation in CI — **DONE (v0.4.0), needs first-release validation**

**Description.** As a federal-civilian procurement officer, I want
CycloneDX 1.6 and SPDX 3.0 SBOMs attached to every release so I can
review the supply chain before approval.

**Acceptance Criteria:**
- [x] [`release.yml`](../.github/workflows/release.yml) generates
      CycloneDX 1.6 JSON via `cargo-cyclonedx` per target.
- [x] Same workflow generates SPDX 3.0 JSON via `anchore/sbom-action`
      (syft under the hood).
- [x] Both SBOM files are attached to the GitHub Release alongside the
      binary, checksum, and signatures.
- [x] [`SECURITY.md`](SECURITY.md) documents how to consume the SBOMs
      with Grype / Trivy / Dependency-Track.
- [ ] **Needs first real release to validate:** confirm the generated
      SBOMs include the NTIA Minimum Elements plus CISA 2025 additions
      (component hash, license, tool name, generation context, software
      producer). The tools used produce these fields by default; this
      is a verification step, not a missing implementation.

#### US-111: Signed releases (Sigstore + GPG) — **DONE (v0.4.0), needs GPG key provisioning**

**Description.** As an Iron Bank reviewer, I want signed release
artifacts with public-log provenance so I can verify origin in an
air-gapped enclave.

**Acceptance Criteria:**
- [x] Release workflow signs each artifact with cosign keyless OIDC via
      `sigstore/cosign-installer@v3` and `cosign sign-blob`. The bundle
      includes the Rekor inclusion proof so verification is offline-
      capable.
- [x] Release workflow GPG-signs each artifact via
      `crazy-max/ghaction-import-gpg@v6` reading
      `secrets.GPG_SIGNING_KEY` + `secrets.GPG_SIGNING_PASSPHRASE`.
- [x] [`SECURITY.md`](SECURITY.md) documents verification commands for
      both methods, including the `--certificate-identity-regexp` pin
      pattern for `cosign verify-blob`.
- [ ] **Operator action required before first release:**
      1. Generate a long-lived GPG signing key offline.
      2. Add `GPG_SIGNING_KEY` and `GPG_SIGNING_PASSPHRASE` to repository
         secrets.
      3. Publish the public key to `keys.openpgp.org` and to the project
         landing page.
      4. Replace `<FINGERPRINT_TO_BE_PUBLISHED_AT_FIRST_RELEASE>` in
         [`SECURITY.md`](SECURITY.md) with the real fingerprint.
- [ ] **Operator action before SLSA L3 audit:** pin every `uses:` line
      in [`release.yml`](../.github/workflows/release.yml) to a commit
      SHA. Tag pins (e.g. `@v4`) are acceptable during bring-up but a
      strict SLSA L3 audit will flag them.

#### US-112: SLSA v1.0 Build L3 provenance — **DONE (v0.4.0)**

**Description.** As a Platform One Big Bang adopter, I want SLSA v1.0
Build L3 in-toto attestations so my pipeline can verify build integrity.

**Acceptance Criteria:**
- [x] Release workflow uses `actions/attest-build-provenance@v2` which
      produces an in-toto attestation with predicate type
      `https://slsa.dev/provenance/v1` (SLSA v1.0).
- [x] Provenance is published as both an OCI referrer and as a workflow
      artifact attached to the GitHub Release.
- [x] Build runs on GitHub-hosted runners (the platform that
      `actions/attest-build-provenance` requires for L3-grade
      attestation).
- [x] [`SECURITY.md`](SECURITY.md) documents `gh attestation verify` as
      the consumer-side verification command.

#### US-113: `SECURITY.md`, `OFFLINE.md`, vulnerability disclosure — **DONE (v0.4.0, pulled forward into Batch D)**

**Description.** As a CISO, I want a documented vulnerability
disclosure policy and offline-deployment statement so I can accept the
tool into an ATO boundary.

**Acceptance Criteria:**
- [x] [`SECURITY.md`](SECURITY.md) covers supported versions, GitHub
      private-advisory + email reporting channel, 3-business-day ack
      SLA, 10-business-day triage SLA, and verification commands for
      SHA-256, cosign, GPG, SLSA, and SBOM.
- [x] [`OFFLINE.md`](OFFLINE.md) enumerates the per-subcommand
      network surface (zero for `validate`, `grammar-info`,
      `corpus-info`), states no-telemetry / no-auto-update, lists
      honored environment variables, and documents `cargo vendor` for
      air-gap builds.
- [x] [`THREAT_MODEL.md`](THREAT_MODEL.md) enumerates five trust
      boundaries (build→release, user inputs→validator,
      `--official-command`→child, filesystem reads/writes,
      validator→consumer) with mitigations and residual risks, plus an
      "out of scope" section.

#### US-114: Section 508 polish — **DONE (v0.4.0)**

**Description.** As a federal user using a screen reader, I want the
default CLI output to be readable without ANSI / color decoration.

**Acceptance Criteria:**
- [x] `--format plain` produces ANSI-free output in the GCC-style
      `path:line:column: severity: code: message` format that screen
      readers and IDEs already understand. Implemented in
      [report.rs](../src/report.rs).
- [x] `NO_COLOR=1` is honored — trivially today since zero ANSI escape
      sequences are emitted in any output format, audited by grep over
      `src/`. Contract documented for the day TTY-aware color is added.
- [x] No color-only signaling: severity is always carried by the
      literal text `error` / `warning` / `info` (lowercase in `plain`,
      uppercase in `text`). Verified by code audit and end-to-end
      smoke test.
- [x] [`docs/accessibility.md`](accessibility.md) includes a draft
      VPAT 2.5 (Revised 508) covering Chapter 3 functional performance
      criteria (302.1–302.9), Chapter 5 software requirements (502, 503,
      504), Chapter 6 support documentation (601–603), and a WCAG 2.0
      Level A/AA cross-reference for the applicable criteria.
- [ ] **Operator action before RFP submission:** review the draft VPAT
      with an accessibility consultant, sign and date, publish as a
      versioned artifact alongside releases.

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

#### US-202: Library loader [EPIC] — **DONE (v0.5.0)**

**Description.** As a SysML v2 model author, I want my model to validate
against `ISQ::Mass`, `SI::kg`, `Geometry::Point`, etc. so I can use the
standard library.

**Acceptance Criteria:**
- [x] Vendored upstream `Systems-Modeling/SysML-v2-Release` at git
      submodule [`vendor/sysml-v2-release/`](../vendor/sysml-v2-release/),
      pinned to release tag `2026-04` (commit
      `9baca5908ca28b53da085de69336fde48420ea8f`). License: EPL-2.0;
      preserved in [`NOTICE.md`](../NOTICE.md).
- [x] [`include_dir!`](../src/library.rs) embeds the full
      `sysml.library/` tree (94 files, ~1.7 MB of textual library
      source) into the binary at compile time. The released binary is
      self-contained — no submodule needed at runtime, no network call.
- [x] `sysml-validate library-info` prints source description (with
      pinned release tag), file count, declaration count, and the full
      list of indexed package names. Supports `--format text|json`.
- [x] The loader builds two indices: `qualified_names` (`Parts::Part`,
      `Items::Item`, ...) and `unqualified_names` (`Part`, `Item`, ...).
      Both consulted in `validate_reference_candidates` for `--strict`.
- [x] `--library-path <dir>` overrides the embedded library with an
      on-disk copy. Useful for testing pre-release library revisions
      against existing models.
- [x] End-to-end verified: a model using `:> Part` and
      `:> Parts::Part` no longer produces SYSML040 false positives;
      a model using `:> CompletelyMadeUp` still does.
- [x] [`REPRODUCING.md`](REPRODUCING.md) and
      [`NOTICE.md`](../NOTICE.md) document the pinned library revision
      so submodule drift becomes a reproducibility failure rather than
      a silent behavioral change.
- [ ] **Deferred to US-203 (next batch):** "library provides a search
      scope for qualified-name resolution" — today the index answers
      yes/no membership; full resolution (returning the resolved
      declaration site, walking specialization chains across packages)
      is the natural follow-up.

#### US-203: Qualified-name resolution [EPIC] — **DONE (v0.6.0), one item deferred**

**Description.** As a SysML v2 model author, I want `import`, `private
import`, `alias`, and `A::B::C` to resolve across files within a
project.

**Acceptance Criteria:**
- [x] Parses every shape of `import` per KerML §8.2.3.4.2:
      MembershipImport (`import A::B::C;`), NamespaceImport
      (`import A::B::*;`), recursive variants (`import A::B::**;`,
      `import A::B::*::**;`), visibility prefixes (`public`, `private`,
      `protected`), and `import all`. Implemented in
      [`imports.rs`](../src/imports.rs).
- [x] Lexer updated to recognize `**` as a single normative KerML token
      (per BNF clause 11.6).
- [x] Cross-file project-wide symbol table (`ProjectIndex`) aggregates
      declarations from every file in the validation run.
      Implemented in [`project.rs`](../src/project.rs).
- [x] `validate_reference_candidates` consults the resolution order:
      (1) declared-in-file, (2) embedded library, (3) project index,
      (4) explicit/wildcard imports against both library and project.
- [x] `SYSML040` message updated to reflect the broader resolution
      path so the user sees that imports and the project were
      consulted.
- [x] End-to-end verified: importing `Engines::Engine` from one file
      resolves `:> Engine` in another; bare `:> CompletelyMadeUp`
      still warns.
- [x] **G.5 follow-up (v0.6.1):** added `:` (typed-usage colon) to the
      reference-marker set. Surfaced while running against the
      `../scamp/` reference model — 18 typed-usage patterns per file
      were previously unchecked. Real-world verification: scamp's
      13-file, 6,015-LOC model now passes `--strict` with `:`
      enabled, and an injected `part animind : AnimindParrt` typo
      is correctly flagged.
- [ ] **Deferred to US-205 batch:** the dedicated `SYSML200`
      "unresolved name" error with full searched-scope chain. Today
      we emit `SYSML040` (warning) with a description of where we
      looked; promoting to an error and detailing the chain is a
      one-flag-and-one-message change once the Pilot rule port lands.
- [ ] **Deferred to differential-test batch (US-207):** running the
      Pilot's `sysml/src/examples/` through both implementations and
      reporting the diff. The project-wide infrastructure for this
      now exists; the harness itself is the natural next batch.

#### US-204: Project manifest support — **PARTIALLY DONE (v0.8.0); KPAR deferred**

**Description.** As a SysML v2 project lead, I want a `.project.json`
file (Sysand-compatible) at the project root so all my files are
validated together.

**Acceptance Criteria:**
- [x] `.project.json` discovery walks up from the working directory.
      Implemented in [`manifest.rs`](../src/manifest.rs).
- [x] Manifest fields parsed: `name`, `version`, `description`, `root`,
      `dependencies[].{name,version,source}`, plus an open `meta`
      passthrough.
- [x] `root` field defines the source root, taking precedence over
      `sysml-validate.toml`'s `project_root` field, which in turn
      takes precedence over the config-file directory.
- [x] Project name and manifest path appear in text and JSON metadata
      blocks (`project:` / `"project"` / `manifest_path`).
- [x] `deny_unknown_fields` rejects malformed manifests at parse time.
- [x] 6 unit tests covering discovery, ascent, `root` resolution,
      dependencies parsing, no-manifest fallback, and unknown-field
      rejection.
- [ ] **Deferred to Phase 2.x:** KPAR archive loading as a dependency
      source. KPAR is a zip-format archive per KerML §10; loading it
      requires a zip dep and a non-trivial archive walker. Sysand's
      dependency-graph resolution model also evolves; pinning to it
      now risks reworking it later.
- [ ] **Deferred to Batch K (real parser):** using manifest
      `dependencies[]` entries to seed cross-project name resolution.
      Today the manifest is discovered and surfaced; the dependencies
      field is parsed but not yet consulted by the resolver.

#### US-205: Port the Pilot's named validator rules [EPIC] — **PARTIALLY DONE (v0.7.0); deeper rules pending US-201**

**Description.** As a SysML v2 model author, I want the same semantic
checks the Pilot runs so my diagnostics are portable.

**Honest scoping.** The Pilot's named rules
(`checkFeatureParameterRedefinition`,
`validateRedefinitionDirectionConformance`,
`checkActionUsageSubactionSpecialization`) require **typed semantic
analysis over an AST** to check things like parameter-order
conformance, port-flow-direction compatibility, and action-body
subaction marking. Without US-201 (real parser), faithful ports of
those exact rules are not achievable. Batch H delivered the subset of
structural rules that ARE achievable on top of the token + project-
index infrastructure built in Batches F + G — high-value catches that
no single-file linter can produce.

**Acceptance Criteria (v0.7.0 scope):**
- [x] **SYSML210 `SpecializationTargetMissing`** — `:>` target
      resolution promoted from warning to error. Token-level
      implementation in [`validate.rs`](../src/validate.rs).
- [x] **SYSML211 `RedefinitionTargetMissing`** — same for `:>>`.
- [x] **SYSML212 `SelfSpecialization`** — `feature x :> x`.
- [x] **SYSML213 `SelfRedefinition`** — `feature x :>> x`.
- [x] **SYSML220 `SpecializationCycle`** — project-wide cycle
      detection via DFS over a specialization graph built during the
      project index pre-pass. Detects cycles that span multiple
      files — the kind of bug that's invisible to per-file linters
      and motivated keeping the project index from Batch G.
      Implementation in [`project.rs`](../src/project.rs).
- [x] Each new rule has fixture-style unit tests + an end-to-end
      smoke test (cross-file cycle injection produces the expected
      `SYSML220` with the full cycle path in the message).
- [x] Verified against the `../scamp/` reference model (13 files,
      295 declarations): all five rules pass clean — confirming the
      rules don't false-fire on a real, well-structured MBSE model.

**Deferred to a later batch (requires US-201 AST or substantial
parsing infrastructure):**
- [ ] `checkFeatureParameterRedefinition` — parameter-order
      conformance in action redefinition.
- [ ] `validateRedefinitionDirectionConformance` — `in`/`out`/`inout`
      compatibility in redefinition.
- [ ] `checkActionUsageSubactionSpecialization` — subaction marking
      in action bodies.
- [ ] Type-conformance checks for `:>` and `:>>` (redefining feature
      type must specialize redefined feature type).
- [ ] Multiplicity-compatibility checks for `:>` and `:>>`.
- [ ] Port-flow-direction compatibility in binding connectors.

**Differential testing against the Pilot** is US-207 — natural next
batch once the structural rules are stable and we have a comparable
diagnostic surface.

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
