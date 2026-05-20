# sysml-validate — Executive Summary

**Capability statement for Flag-officer / SES decision audiences.**

| Field | Value |
|---|---|
| Product | `sysml-validate` |
| Version | 0.15.0 |
| Last updated | 2026-05-19 |
| Read time | ~3 minutes |

---

## What it is

`sysml-validate` is a small, fast, self-contained command-line validator
for **SysML v2 and KerML** — the OMG formal modeling standards
(`formal/26-03-02` and `formal/26-03-01`, September 2025) that DoD
Digital Engineering programs, NASA mission directorates, and federal
civilian acquisitions are increasingly required to use under
[DoDI 5000.97 Digital Engineering](https://www.esd.whs.mil/Portals/54/Documents/DD/issuances/dodi/500097p.PDF)
and OMB [M-26-05](https://www.whitehouse.gov/wp-content/uploads/2026/01/M-26-05-Adopting-a-Risk-based-Approach-to-Software-and-Hardware-Security.pdf).

The product is **one statically-linked binary** (~10 MB) plus the
documents bundled alongside it. No installer, no runtime dependencies,
no network access, no telemetry, no auto-update.

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
- **IL2 / IL4 / IL5 compatible.** No network calls, no telemetry, no
  auto-update; the binary is acceptable inside SIPR / NIPR enclaves.
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

`sysml-validate` is a **preflight CI gate**, not a replacement for the
OMG Pilot Implementation. Deep semantic checks (constraint evaluation,
expression evaluation, full conformance) are delegated to the Pilot
via the hardened `--backend official` channel when the consumer needs
them. The roadmap and trade-offs are explicit in
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
