# Conformance & Compliance Index

This index is the **first document a government reviewer should
read**. It maps reviewer role → the specific document(s) in this
release bundle that answer that reviewer's questions.

Every artifact referenced here is included verbatim in the release
bundle (`sysml-validate-<version>-<target>/docs/`).

| Field | Value |
|---|---|
| Product | `sysml-validate` |
| Version | 0.16.0 |
| Bundle date | 2026-05-20 |
| Bundle hash | See `BUNDLE-MANIFEST.txt` in the release archive |

---

## By reviewer role

### SCRM (Supply-Chain Risk Management) reviewer

You need to verify provenance, integrity, and dependency composition
before admitting the artifact.

| Question | Document |
|---|---|
| What's in the binary? | `sysml-validate-<target>.cdx.json` (CycloneDX 1.6 SBOM); `sysml-validate-<target>.spdx.json` (SPDX 3.0 SBOM) |
| Who built it, how, and where? | `provenance.intoto.jsonl` (SLSA v1.0 Build L3 in-toto attestation) |
| How do I verify the signatures? | [`docs/SECURITY.md`](../SECURITY.md) — cosign + GPG verification recipes |
| Is the build reproducible? | [`docs/REPRODUCING.md`](../REPRODUCING.md) — byte-identical rebuild recipe; the release workflow runs `diffoscope` between two independent rebuilds and fails on diff |
| What's the disclosure / patch policy? | [`docs/SECURITY.md`](../SECURITY.md) §"Reporting vulnerabilities" |

### ATO (Authority to Operate) reviewer

You need to align the artifact with the system's authorization
boundary and accreditation package.

| Question | Document |
|---|---|
| What NIST 800-53 Rev 5 controls does the tool support? | [`docs/compliance/nist-800-53-mapping.md`](nist-800-53-mapping.md) — SA-11, SA-15, SI-7, SR-3/4/11, CM-7, AU-2, AU-12 with explanations; partially-supported and out-of-scope controls explicitly enumerated |
| Does it satisfy SSDF? | [`docs/compliance/ssdf-mapping.md`](ssdf-mapping.md) — NIST SP 800-218 PO/PS/PW/RV evidence mapped to repo artifacts |
| What's the network surface? | [`docs/OFFLINE.md`](../OFFLINE.md) — zero network calls in `validate`, `grammar-info`, `corpus-info`, `library-info`, `lsp`; no telemetry, no auto-update |
| What's the threat model? | [`docs/THREAT_MODEL.md`](../THREAT_MODEL.md) — five trust boundaries with mitigations and residual risks |
| Does it run in IL2 / IL4 / IL5? | Yes. See [`docs/compliance/nist-800-53-mapping.md`](nist-800-53-mapping.md) §"DoD Cloud SRG impact-level guidance" |

### Airworthiness / safety-critical reviewer (DO-178C / DO-330)

You need a qualification kit so this tool can discharge a verification
objective in a DAL A–D program.

| Question | Document |
|---|---|
| What's the qualification posture? | [`docs/compliance/do-330-qualification-kit/`](do-330-qualification-kit/) — Criterion 3 (verification tool, no replacement of activity); TQL-5 target |
| What does the kit contain? | TOR (Tool Operational Requirements), TQP (Tool Qualification Plan), TQA (Tool Qualification Agreement), TCMP (Tool Configuration Management Plan), TVCP (Tool Verification Cases & Procedures), TVCR (Tool Verification Cases & Results), TAS (Tool Accomplishment Summary). All templates; adopter completes against the specific TOR |
| What evidence does the kit cross-reference? | [`docs/SECURITY.md`](../SECURITY.md), [`docs/REPRODUCING.md`](../REPRODUCING.md), [`docs/THREAT_MODEL.md`](../THREAT_MODEL.md), [`docs/differential-corpus-report.md`](../differential-corpus-report.md) |
| How was the rule catalog validated? | Differential corpus harness runs against the OMG `examples` (95 files) and `validation` (56 files) corpora nightly in CI; see [`docs/differential-corpus-report.md`](../differential-corpus-report.md) |

### NASA NPR 7150.2D reviewer

You need a tool validation report for a Class A/B/C software project.

| Question | Document |
|---|---|
| What's the tool validation evidence? | [`docs/compliance/nasa-npr-7150-2d-tool-validation.md`](nasa-npr-7150-2d-tool-validation.md) — aligned with §4.4.8 and §4.5.6; intended use, project-use context, validation evidence, validation approach, operational limits, re-validation triggers, sign-off section |
| Is the tool acceptable for Class A? | The template documents the operational limits and the Class A caveat. The project's software assurance reviewer makes the determination per program context |

### CMMC 2.0 Level 2 contractor

You need a deployment recipe that does not undo your existing controls.

| Question | Document |
|---|---|
| How do I deploy without breaking my CMMC posture? | [`docs/compliance/cmmc-l2-deployment.md`](cmmc-l2-deployment.md) — pre-deployment verification, install model, control-domain mapping (AC, AU, CM, IA, SI), air-gap deployment, hardened-container example |
| What controls does the tool itself touch? | AC, AU, CM, IA, SI — the deployment guide explains how each is supported (or not affected) |

### Section 508 / accessibility reviewer

You need confirmation that the CLI is usable with assistive technology
and a VPAT for ICT procurement.

| Question | Document |
|---|---|
| What's the Section 508 conformance posture? | [`docs/accessibility.md`](../accessibility.md) — Chapter 3 functional performance, Chapter 5 software, Chapter 6 docs, plus WCAG 2.0 A/AA cross-reference |
| Is there a VPAT? | Yes, a **draft** VPAT 2.5 (Rev 508) is embedded in [`docs/accessibility.md`](../accessibility.md). The signed VPAT is gated on third-party consultant review (PRD §10 / O-5 — operator action before RFP submission) |
| What does the `--format plain` mode provide? | GCC-style one-diagnostic-per-line ANSI-free output that screen readers and IDE problem matchers consume directly. Documented in [`TECH_MANUAL.md`](../TECH_MANUAL.md) §6.2 |

### Program manager / first-line decision-maker

| Question | Document |
|---|---|
| Why should we use this? | [`docs/EXECUTIVE_SUMMARY.md`](../EXECUTIVE_SUMMARY.md) — one page, Flag / SES audience |
| What's the roadmap? | [`docs/PRD-government-readiness.md`](../PRD-government-readiness.md) — full PRD with phase plan |
| What's done vs. what's next? | The PRD's "Batch index" table at the top of the document is the at-a-glance status |

### End user (operator, CI engineer, model author)

| Question | Document |
|---|---|
| How do I use it? | [`docs/TECH_MANUAL.md`](../TECH_MANUAL.md) — full end-user manual |
| What does the CLI accept? | `sysml-validate --help`, `sysml-validate <subcommand> --help`, or `TECH_MANUAL.md` §4 |
| What does each rule mean? | `TECH_MANUAL.md` §7 (rule catalog with remediation) |

---

## File inventory (release bundle layout)

A release bundle (`sysml-validate-<version>-<target>.tar.gz` or
`.zip`) expands to:

```
sysml-validate-<version>-<target>/
├── BUNDLE-MANIFEST.txt           # path → SHA-256 of every file in this archive
├── VERSION                       # one line: the version string
├── LICENSE                       # MIT license text
├── NOTICE.md                     # EPL-2.0 attribution for vendored library
├── CHANGELOG.md                  # if present in the source tree
├── bin/
│   ├── sysml-validate(.exe)      # the binary
│   ├── sysml-validate.sha256
│   ├── sysml-validate.cosign.bundle
│   └── sysml-validate.asc        # GPG signature (if key was available at build)
├── sbom/
│   ├── sysml-validate.cdx.json   # CycloneDX 1.6
│   └── sysml-validate.spdx.json  # SPDX 3.0
├── attestation/
│   └── provenance.intoto.jsonl   # SLSA v1.0 Build L3 in-toto attestation
├── skills/
│   └── sysml-validate/           # agentskills.io-spec agent skill (US-311)
│       ├── SKILL.md              # frontmatter + action-oriented instructions
│       └── references/
│           ├── rule-catalog.md       # per-SYSMLxxx remediation table
│           ├── suppression-syntax.md # directive grammar reference
│           └── cicd-recipes.md       # per-platform CI recipes
└── docs/
    ├── EXECUTIVE_SUMMARY.md
    ├── TECH_MANUAL.md
    ├── PRD-government-readiness.md
    ├── SECURITY.md
    ├── OFFLINE.md
    ├── THREAT_MODEL.md
    ├── REPRODUCING.md
    ├── accessibility.md
    ├── differential-corpus-report.md
    └── compliance/
        ├── INDEX.md                          (this file)
        ├── ssdf-mapping.md
        ├── nist-800-53-mapping.md
        ├── cmmc-l2-deployment.md
        ├── nasa-npr-7150-2d-tool-validation.md
        └── do-330-qualification-kit/
            ├── README.md
            ├── tor-template.md
            ├── tqp-template.md
            ├── tqa-template.md
            ├── tcmp-template.md
            ├── tvcp-template.md
            ├── tvcr-template.md
            └── tas-template.md
```

### Agent skill (skills/sysml-validate/)

The bundle includes a `SKILL.md` per the
[agentskills.io](https://agentskills.io) open specification. Agentic
clients (Claude Code, Cursor, Gemini CLI, OpenCode, Junie,
OpenHands, GitHub Copilot, Goose, Amp, Roo Code, and others) can
drop the `skills/sysml-validate/` directory into their agent's
skills path and the agent gains project-specific knowledge of the
diagnostic taxonomy, suppression syntax, and CI integration
patterns. This is an **integration accelerator for AI-augmented
adopters** — analogous to shipping an LSP capability spec for
editor-augmented adopters.

`BUNDLE-MANIFEST.txt` lists every file in the bundle with its SHA-256
hash. Verify it with the recipe in
[`docs/SECURITY.md`](../SECURITY.md) §"Verifying a release bundle."
