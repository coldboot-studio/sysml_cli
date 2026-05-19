# DO-330 / EUROCAE ED-215 Tool Qualification Kit Skeleton

This directory contains template documents for qualifying
`sysml-validate` as a DO-330 verification tool (target: **TQL-5**).
The templates are intended for adaptation by a project pursuing
DO-178C / ED-12C airworthiness certification that uses
`sysml-validate` to discharge a verification objective.

## Tool Criterion and TQL determination

DO-330 §6.1 defines three tool criteria and five TQL levels.
`sysml-validate` is classified as follows:

- **Tool Criterion**: Criterion 3 — "the tool could fail to detect
  an error in its outputs."
- **Software Level**: applicable up to DAL A (most stringent).
- **Tool Qualification Level**: TQL-5 (verification tool, lightest
  qualification per DO-330 §11.5).

A user deploying `sysml-validate` to discharge a development-related
objective (i.e., generating airborne software outputs) would need
TQL-4 or higher, which requires additional verification of the tool
itself. That is not the intended use case.

## Required artifacts for TQL-5

Per DO-330 Annex A, a TQL-5 qualification kit comprises:

1. **Tool Operational Requirements (TOR)** — what the tool must do
   in the operational context of the project using it.
   See [`TOR-template.md`](TOR-template.md).
2. **Tool Qualification Plan (TQP)** — planning document covering
   tool qualification activities, organizational responsibilities,
   environment, and lifecycle data.
   See [`TQP-template.md`](TQP-template.md).
3. **Tool Quality Assurance Plan (TQA)** — quality assurance for
   the tool qualification process itself.
   See [`TQA-template.md`](TQA-template.md).
4. **Tool Configuration Management Plan (TCMP)** — configuration
   management for the tool artifacts (which version, which
   environment, which inputs).
   See [`TCMP-template.md`](TCMP-template.md).
5. **Tool Verification Cases and Procedures (TVCP)** — the test
   cases and procedures that verify the tool meets its TOR.
   See [`TVCP-template.md`](TVCP-template.md).
6. **Tool Verification Cases and Results (TVCR)** — execution
   results of the TVCP.
   See [`TVCR-template.md`](TVCR-template.md).
7. **Tool Accomplishment Summary (TAS)** — closure summary.
   See [`TAS-template.md`](TAS-template.md).

## Status

This kit is a **template skeleton**, not a completed qualification
package. The maintainer position is:

- Skeleton + filled-in TOR for one representative ruleset are
  available so an adopting project can see the shape of the work.
- The remaining templates (TQP, TQA, TCMP, TVCP, TVCR, TAS) are
  outlines, not signed deliverables.
- A complete kit is filled in **per adopting project**, since TOR
  language depends on the project's specific use of the tool
  (which rules, which output formats, which lifecycle phase).

Adopting projects should expect to invest project-specific effort
to complete the kit; the templates reduce that effort by providing
the scaffolding and references back to the implementation evidence.

## Implementation evidence referenced by these templates

The templates reference these artifacts as evidence:

- [`../../SECURITY.md`](../SECURITY.md) — release verification.
- [`../../REPRODUCING.md`](../REPRODUCING.md) — bit-reproducible build.
- [`../../THREAT_MODEL.md`](../THREAT_MODEL.md) — trust boundaries.
- [`../../differential-corpus-report.md`](../differential-corpus-report.md)
  — verification against the OMG corpus.
- Test reports: `cargo test` produces the unit-test results;
  `cargo test --test differential -- --ignored` produces the
  corpus comparison.
- SBOMs (CycloneDX 1.6 + SPDX 3.0) attached to each release.
- SLSA v1.0 in-toto build provenance.

## Related

- [`../ssdf-mapping.md`](../ssdf-mapping.md) — NIST SSDF mapping
- [`../nist-800-53-mapping.md`](../nist-800-53-mapping.md) —
  NIST 800-53 Rev 5
- [`../cmmc-l2-deployment.md`](../cmmc-l2-deployment.md) — CMMC L2
