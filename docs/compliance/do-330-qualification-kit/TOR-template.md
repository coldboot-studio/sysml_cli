# Tool Operational Requirements — TEMPLATE

> **Adoption note.** Replace each `<placeholder>` with the
> project-specific language. The "Sample TOR" sections below contain
> language suitable for a project that uses `sysml-validate` to check
> SysML v2 model structural integrity as part of a model-based
> requirements verification activity. Adapt per your usage.

## TOR.1 Identification

- **Tool name**: `sysml-validate`
- **Tool version**: `<version under qualification, e.g. 0.11.0>`
- **Rule catalog version**: `<value of RULE_CATALOG_VERSION at that release>`
- **Embedded SysML v2 standard library tag**: `<OMG release tag, e.g. 2026-04>`
- **Tool source**: <repository URL>
- **Tool license**: MIT (validator), EPL-2.0 (embedded standard library)
- **Adopting project**: `<project name>`
- **Software Level**: `<DAL A | B | C | D>`
- **Tool Criterion**: 3 (verification tool)
- **Tool Qualification Level**: TQL-5

## TOR.2 Operational Environment

- **Host operating systems**: `<list approved hosts, e.g. RHEL 9, Windows Server 2022>`
- **Architecture**: `<x86_64, aarch64>`
- **Invocation context**: command-line, non-interactive
- **Network access**: not required (validator makes no network calls;
  see [`OFFLINE.md`](../../OFFLINE.md))
- **Pre-conditions for execution**: source models in `.sysml` or
  `.kerml` form on the local filesystem
- **Post-conditions**: SARIF / JUnit / JSON / text diagnostic report
  on stdout

## TOR.3 Functional Requirements

The following requirements describe the tool's behavior in the
operational context. Each requirement is verified by one or more test
cases in the TVCP.

**TOR-FR-001.** Given a syntactically well-formed SysML v2 textual
model, the tool shall report zero error-level diagnostics.

**TOR-FR-002.** Given a SysML v2 textual model containing a
duplicate member-name within a single lexical scope, the tool shall
emit a `SYSML041` diagnostic at error severity referencing the
offending declaration.

**TOR-FR-003.** Given a SysML v2 textual model containing a `:>` or
`specializes` reference whose target does not resolve through (a)
declared-in-file names, (b) the embedded SysML v2 standard library,
(c) project-wide declarations, or (d) explicit imports, the tool
shall emit a `SYSML210` diagnostic at error severity.

**TOR-FR-004.** Given a SysML v2 textual model whose `:>` graph
contains a cycle of two or more nodes (across any file in the
validation run), the tool shall emit a `SYSML220` diagnostic at
error severity, with the cycle path included in the message.

**TOR-FR-005.** The tool shall emit a per-run metadata block on
every invocation, containing: tool name, tool version, rule catalog
version, RFC 3339 UTC timestamp, backend identity, and ruleset flags.

**TOR-FR-006.** The tool's `validate` subcommand shall make no
outbound network connections.

**TOR-FR-007.** With `--format sarif`, the tool shall emit a SARIF
2.1.0 log conforming to the OASIS schema, with stable
`partialFingerprints.diagnosticHash/v1` per result.

**TOR-FR-008.** With `--baseline <prior.sarif>`, the tool shall
classify each finding as `new` or `unchanged` against the prior
SARIF log and shall not fail the build on `unchanged` findings.

`<Add project-specific functional requirements here.>`

## TOR.4 Operational Use Cases

**UC-1.** A model is committed to a project repository. CI invokes
`sysml-validate validate <model-directory> --ci --baseline
<baseline.sarif>`. The pipeline interprets a non-zero exit code as a
verification failure.

**UC-2.** A new model file is added to the project. The developer
runs `sysml-validate validate <new-file>.sysml --strict` locally
and addresses each diagnostic before committing.

`<Add additional use cases per your workflow.>`

## TOR.5 Tool Operational Limits

The tool **does not** today verify the following classes of properties
(see [`differential-corpus-report.md`](../../differential-corpus-report.md)
for current state); deployments must not rely on it for these:

- Full SysML v2 OCL well-formedness constraints (~several hundred in
  the OMG spec; only a subset of structural ones are implemented).
- Expression evaluation for `calc` / `assert` / `require` constraints.
- Type / multiplicity / port-flow-direction conformance in feature
  redefinition (deferred to a future release; see PRD US-205).
- v1→v2 transformation correctness (out of scope).
- Faithful side-by-side equivalence with the OMG Pilot Implementation
  (the differential harness is presumptive today; see
  [`differential-corpus-report.md`](../../differential-corpus-report.md)
  methodology section).

## TOR.6 Configuration Items

For the qualification to remain valid, the following items shall be
held constant unless re-qualification is performed:

- Tool binary (verified by SHA-256 against published checksum)
- Tool version (verified by `sysml-validate --version`)
- Rule catalog version (verified in the metadata block of any run)
- Embedded SysML v2 standard library tag (verified by
  `sysml-validate library-info`)
- Configuration file (`sysml-validate.toml`) — if used, its content
  is a configuration item

## TOR.7 References

- DO-330 / ED-215, "Software Tool Qualification Considerations"
- DO-178C / ED-12C, the parent process this qualification supports
- [`../../PRD-government-readiness.md`](../../PRD-government-readiness.md)
  — full feature roadmap
- [`../../SECURITY.md`](../../SECURITY.md)
- [`../../THREAT_MODEL.md`](../../THREAT_MODEL.md)
- [`../../differential-corpus-report.md`](../../differential-corpus-report.md)
