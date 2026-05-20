# sysml-validate — Executive Summary

**Capability statement for Flag-officer / SES decision audiences.**

| Field | Value |
|---|---|
| Product | `sysml-validate` |
| Version | 0.16.0 |
| Last updated | 2026-05-20 |
| Read time | ~3 minutes |

---

## What it is

`sysml-validate` is a small, fast, self-contained command-line validator
for **SysML v2 and KerML** — the OMG formal modeling standards
(`formal/26-03-02` and `formal/26-03-01`, finalized September 2025;
editorially revised March 2026 for ISO Fast-Track) that DoD Digital
Engineering programs, NASA mission directorates, and federal civilian
acquisitions are increasingly adopting under
[DoDI 5000.97 Digital Engineering](https://www.esd.whs.mil/Portals/54/Documents/DD/issuances/dodi/500097p.PDF).
DoDI 5000.97 mandates digital-engineering practice and the use of
digital models as authoritative artifacts; it does not name a single
modeling language, but SysML v2 is the modern OMG-standard answer
that programs are converging on. The tool's supply-chain and release
artifacts are independently aligned with OMB
[M-26-05](https://www.whitehouse.gov/wp-content/uploads/2026/01/M-26-05-Adopting-a-Risk-based-Approach-to-Software-and-Hardware-Security.pdf)'s
risk-based assurance posture (SBOMs, SLSA provenance, signatures) so
agencies opting in under that memorandum's tailored approach have the
evidence they need.

The product is **one single-file executable** (~10 MB) plus the
documents bundled alongside it. No installer, no application runtime
or service dependency, no network access, no telemetry, no
auto-update. On Linux the binary dynamically links the platform's
glibc (the standard `x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-gnu`
targets); macOS and Windows binaries are correspondingly linked
against the platform's standard libraries. A fully static `musl`
build is on the roadmap for tightly-constrained enclaves.

## What's different

`sysml-validate` does not compete with full modeling tools — Cameo /
CATIA Magic 2026.x, Eclipse SysON, the OMG Pilot Implementation,
Sensmetry's SysIDE, or OpenMBEE / Flexo all serve adjacent niches
(authoring, visualization, formal V&V, API workflows). It occupies a
narrower, currently-unoccupied position:

- **Native single-file CI gate.** No JVM, no Docker, no Maven, no
  Eclipse. Cold start in tens of milliseconds. The closest CI-focused
  open-source alternative (Westfall-io `windtrader`) is a Python
  wrapper around the Java Pilot Implementation; it has no SARIF or
  JUnit output and inherits the JVM start-up cost.
- **Structured findings out of the box.** SARIF 2.1.0, JUnit XML,
  GCC-style plain text, JSON, and human text — every format the
  consumer's existing pipeline already speaks. No competing SysML v2
  tool publishes SARIF.
- **Baseline / diff mode for adoption-without-rewrite.** Existing
  large models can be onboarded without first fixing every legacy
  finding.
- **Audited suppression directives.** Inline disables surface in SARIF
  as `suppressions[].kind = "inSource"` audit records rather than
  silently dropping diagnostics.
- **Government release bundle.** Per-target archive ships the signed
  binary alongside CycloneDX 1.6 + SPDX 3.0 SBOMs, SLSA v1.0 in-toto
  provenance, cosign + GPG signatures, the reproducible-build recipe,
  and the full compliance pack (NIST SSDF, 800-53 Rev 5, CMMC L2,
  DO-330 TQL-5 templates, NPR 7150.2D template, Section 508 VPAT
  draft) — verified end-to-end via a `BUNDLE-MANIFEST.txt` with a
  SHA-256 of every file.
- **Hardened delegation, not pretend-replacement.** Deep semantic
  checks are routed to the OMG Pilot via a shell-injection-safe
  `--backend official` channel, with a `--timeout` kill switch.

## What problem it solves

Federal MBSE programs that adopt SysML v2 inherit a quality-control gap:
the OMG Pilot Implementation is the reference oracle but is heavy
(Java/Maven, multi-second startup, GUI-oriented), and most CI pipelines
need a fast preflight check that runs in seconds and emits findings in
formats their existing tools already consume.

`sysml-validate` fills that gap. It catches structural and reference
errors in CI before they reach review, emits findings in **SARIF
2.1.0** (the lingua franca of GitHub Advanced Security, GitLab
Ultimate, Iron Bank, SonarQube, Azure DevOps), JUnit XML (Jenkins,
GitLab), plain GCC-style output (screen readers, IDE problem
matchers), JSON, and the default human-readable text.

## Trust posture (what ships with every release)

| Artifact | Standard / authority |
|---|---|
| Sigstore (cosign) keyless signature, Rekor-logged | Sigstore / OpenSSF |
| Detached GPG signature (long-lived offline key) | RFC 4880 |
| SHA-256 checksum | NIST FIPS 180-4 |
| CycloneDX 1.6 SBOM | OWASP / ECMA-424 |
| SPDX 3.0 SBOM | ISO/IEC 5962 |
| SLSA v1.0 Build Level 3 in-toto provenance | SLSA / OpenSSF |
| Reproducible build (byte-identical rebuild on independent runner) | Reproducible Builds project; verified per release by `diffoscope` |
| Conformance pack: NIST SP 800-218 SSDF, 800-53 Rev 5, CMMC 2.0 L2, DO-330 TQL-5 templates, NASA NPR 7150.2D template, Section 508 VPAT 2.5 draft | NIST / DoD / FAA / NASA / Access Board |

Each item is **verifiable by the consumer** — no trust in the
project office is required. The full verification recipe is in
[`SECURITY.md`](SECURITY.md).

## Deployment model

- **Air-gap-capable.** The embedded OMG SysML v2 standard library
  (release tag `2026-04`) is compiled into the binary; no submodule
  download is needed at runtime. See [`OFFLINE.md`](OFFLINE.md).
- **Designed for IL2 / IL4 / IL5 enclave deployment** (no network
  calls, no telemetry, no auto-update). Actual deployability into
  any specific enclave or onto SIPR / NIPR is an Authorizing
  Official decision subject to local ATO and security review; the
  tool removes the technical obstacles, not the authority decision.
  See [`compliance/cmmc-l2-deployment.md`](compliance/cmmc-l2-deployment.md).
- **Reproducible by independent rebuild.** A defense prime can rebuild
  the binary from the published source revision and toolchain and
  obtain a byte-identical artifact. See
  [`REPRODUCING.md`](REPRODUCING.md).
- **Editor-friendly.** A built-in Language Server (`sysml-validate
  lsp`) speaks LSP 3.x over stdio; VS Code, Neovim, Helix, and Emacs
  pick up diagnostics live as the user types.

## Evidence portfolio

The release bundle (see [`compliance/INDEX.md`](compliance/INDEX.md))
contains every document a procurement review would request:

- **Supply chain:** SBOMs (CycloneDX + SPDX), SLSA provenance,
  signatures, reproducible-build recipe.
- **Security posture:** [`SECURITY.md`](SECURITY.md),
  [`OFFLINE.md`](OFFLINE.md), [`THREAT_MODEL.md`](THREAT_MODEL.md).
- **NIST mappings:** [SSDF](compliance/ssdf-mapping.md),
  [800-53 Rev 5](compliance/nist-800-53-mapping.md).
- **DoD adoption:** [CMMC L2 deployment](compliance/cmmc-l2-deployment.md).
- **Safety-critical pathway:** [DO-330 TQL-5 qualification kit](compliance/do-330-qualification-kit/)
  (templates for adopter completion).
- **NASA pathway:** [NPR 7150.2D tool validation template](compliance/nasa-npr-7150-2d-tool-validation.md).
- **Accessibility:** [Section 508 VPAT 2.5 draft](accessibility.md).
- **End-user material:** [`TECH_MANUAL.md`](TECH_MANUAL.md).

## What it is not

`sysml-validate` is a **preflight CI gate**, not a full SysML v2
conformance validator and not a replacement for the OMG Pilot
Implementation. Deep semantic checks (constraint evaluation,
expression evaluation, full OCL well-formedness) are delegated to the
Pilot via the hardened `--backend official` channel when the consumer
needs them.

**Known limitation on legacy corpora.** The native backend's
token-based recognizer currently produces a high false-positive rate
on the OMG-curated `examples` corpus (~95% as of v0.16.0, attributable
to documented token-level limitations awaiting full tree-sitter parser
migration in US-201). On a well-structured new project (verified
against the `scamp` reference: 13 files, 6,015 LOC, 295 declarations)
it passes clean. For adoption against an existing large model, use
the `--baseline` workflow or `--backend official` until the parser
migration is complete. The honest snapshot is in
[`differential-corpus-report.md`](differential-corpus-report.md); the
roadmap and trade-offs are in
[`PRD-government-readiness.md`](PRD-government-readiness.md) §4 and
§6.

## Next steps for adoption

1. **Pull a tagged release** from GitHub. Verify the SHA-256, the
   cosign bundle, and the GPG signature using the recipe in
   [`SECURITY.md`](SECURITY.md).
2. **Review the conformance bundle** — start at
   [`compliance/INDEX.md`](compliance/INDEX.md). It maps your role
   (SCRM, ATO, airworthiness, NASA, CMMC L2, Section 508) to the
   specific document that answers your questions.
3. **Pilot deploy** in one CI pipeline using the recipes in
   [`TECH_MANUAL.md`](TECH_MANUAL.md) §11. The tool integrates with
   GitHub Actions, GitLab CI, Jenkins, Azure DevOps, and Iron Bank /
   Platform One via SARIF.
4. **For airworthiness / safety-critical use:** complete the DO-330
   TQL-5 kit templates against your specific Tool Operational
   Requirement.

## Point of contact

This is an open-source project under the MIT license, with vulnerability
disclosure handled per [`SECURITY.md`](SECURITY.md). For acquisition or
sustainment conversations, contact the maintainer listed in the
project's GitHub repository.

---

*SysML® is a registered trademark and KerML™ is a trademark of Object
Management Group, Inc. `sysml-validate` is an independent third-party
tool and is not produced, endorsed, certified, or affiliated with OMG.
See [`NOTICE.md`](../NOTICE.md) for the full attribution of third-party
trademarks and the EPL-2.0-licensed SysML v2 standard library
redistributed with this binary.*
