# NASA NPR 7150.2D Tool Validation Report — TEMPLATE

> Project-specific completion required.
>
> NASA NPR 7150.2D §4.4.8 and §4.5.6 require that software test and
> analysis tools used to satisfy Class A–C software requirements be
> validated for intended use, with documented evidence keyed to
> NASA-STD-8739.8 and NASA-HDBK-2203. This template is the
> corresponding evidence document for `sysml-validate`.

## 1. Tool Identification

- **Tool name**: `sysml-validate`
- **Tool version under validation**: `<e.g. 0.11.0>`
- **Tool category**: Software analysis tool
- **Tool function**: Static validation of SysML v2 and KerML textual
  models — lexical, syntactic, name-resolution, structural checks
- **Source**: <repository URL>
- **License**: MIT (validator), EPL-2.0 (embedded standard library)

## 2. Project Use Context

- **Project**: `<project name>`
- **Software Class**: `<A / B / C / D>` per NPR 7150.2D Appendix E
- **Function in project**: `<e.g., validation of MBSE artifacts
  prior to inclusion in flight-software requirements baseline>`
- **Lifecycle phase**: `<e.g., requirements analysis,
  design verification>`

## 3. Intended Use Statement

`sysml-validate` is used to verify that SysML v2 / KerML model
artifacts meet the structural rules enumerated in the project's
chosen rule catalog. It is **not** used to:

- Verify functional correctness of executable code
- Verify dynamic behavior of models (e.g., simulation results)
- Replace human review of model semantics
- Verify compliance with project-specific safety requirements
  beyond the structural rules listed in the rule catalog

## 4. Validation Evidence

| Evidence | Reference |
|---|---|
| Unit test results (116 tests) | `cargo test` output captured at release time |
| Differential corpus results | [`../../differential-corpus-report.md`](../../differential-corpus-report.md) |
| Service experience | Public repository activity; OMG release-corpus and external project (scamp) testing |
| Reproducible build | [`../../REPRODUCING.md`](../../REPRODUCING.md) |
| Threat model | [`../../THREAT_MODEL.md`](../../THREAT_MODEL.md) |
| Vulnerability response | [`../../SECURITY.md`](../../SECURITY.md) |
| SBOM | CycloneDX 1.6 + SPDX 3.0 attached to each release |
| Build provenance | SLSA v1.0 in-toto attestation per release |

## 5. Validation Approach

The tool is validated by:

1. Executing the full test suite on the release commit and recording
   the results.
2. Executing the differential corpus harness against the OMG SysML v2
   release corpus and recording the histogram of findings against the
   committed baseline.
3. Confirming the byte-for-byte reproducible-build recipe succeeds
   (independent rebuild matches the published SHA-256).
4. Verifying the Sigstore and GPG signatures on the release artifacts.

## 6. Validation Result

`<PASS | FAIL | CONDITIONAL>`

`<Detail any conditions, limitations, or non-conformances.>`

## 7. Operational Limits

- The tool does not implement the full SysML v2 OCL well-formedness
  constraint set. See [`../../PRD-government-readiness.md`](../../PRD-government-readiness.md)
  PRD US-205 and the differential corpus report for current coverage.
- The tool does not evaluate expressions, constraints, or simulation.
- For Class A applications, the project shall NOT rely solely on this
  tool for safety-critical model verification; combine with manual
  review and the OMG Pilot Implementation as an independent check.

## 8. Re-validation Triggers

Re-validation shall be performed when:
- The tool version changes
- The rule catalog version changes (recorded in
  [`../../../src/report.rs`](../../../src/report.rs)
  `RULE_CATALOG_VERSION`)
- The embedded SysML v2 standard library tag changes
- The project's intended use materially changes

## 9. Reviewer Sign-off

- Software Engineering: __________________________ Date: __________
- Software Quality:     __________________________ Date: __________
- Project Manager:      __________________________ Date: __________

## 10. References

- NPR 7150.2D, NASA Software Engineering Requirements
- NASA-STD-8739.8, Software Assurance and Software Safety Standard
- NASA-HDBK-2203, NASA Software Engineering Handbook
- [`../ssdf-mapping.md`](../ssdf-mapping.md)
- [`../nist-800-53-mapping.md`](../nist-800-53-mapping.md)
