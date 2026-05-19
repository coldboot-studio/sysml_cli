# Tool Qualification Plan — TEMPLATE OUTLINE

> Project-specific completion required.

## 1. Identification
- Tool, version, TQL: see [`TOR-template.md`](TOR-template.md) §TOR.1
- Adopting project, software level, DAL

## 2. Tool Qualification Activities
- Activity 1: TOR development → [`TOR-template.md`](TOR-template.md)
- Activity 2: TVCP development → [`TVCP-template.md`](TVCP-template.md)
- Activity 3: TVCP execution → [`TVCR-template.md`](TVCR-template.md)
- Activity 4: TAS preparation → [`TAS-template.md`](TAS-template.md)

## 3. Organizational Responsibilities
- Tool qualification authority
- Tool integrator
- Tool user

## 4. Tool Qualification Environment
- Host OS, architecture
- Reproducible-build recipe → [`REPRODUCING.md`](../../REPRODUCING.md)
- Configuration items → [`TOR-template.md`](TOR-template.md) §TOR.6

## 5. Tool Lifecycle Data
- Source repository, branch, tags
- Build provenance: SLSA v1.0 in-toto attestation per release
- SBOM: CycloneDX 1.6 + SPDX 3.0

## 6. Transition Criteria
- Successful TVCR
- Signed TAS
- Configuration baseline established

## 7. Tool Quality Assurance
- See [`TQA-template.md`](TQA-template.md)

## 8. Tool Configuration Management
- See [`TCMP-template.md`](TCMP-template.md)

## 9. Schedule and Resources
- Project-specific

## 10. Additional Considerations
- Service experience: corpus runs documented in
  [`../../differential-corpus-report.md`](../../differential-corpus-report.md)
- Tool integration: the tool is invoked as a child process from CI
  or developer workstations; no in-process integration is performed.
