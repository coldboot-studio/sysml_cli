# Tool Verification Cases and Procedures — TEMPLATE OUTLINE

> Each test case below maps to a TOR-FR requirement in
> [`TOR-template.md`](TOR-template.md). Project-specific test cases
> may extend this list.

## Test Case TC-001 (verifies TOR-FR-001)
- **Procedure:** Construct or select a known well-formed SysML v2
  model. Invoke `sysml-validate validate <model>` with default flags.
- **Expected:** Exit code 0, no error-level diagnostics in output.

## Test Case TC-002 (verifies TOR-FR-002)
- **Procedure:** Construct a model containing two declarations with
  identical names in the same lexical scope. Invoke
  `sysml-validate validate <model>`.
- **Expected:** Exit code 1; output contains a `SYSML041` diagnostic
  with position pointing at the offending declaration.

## Test Case TC-003 (verifies TOR-FR-003)
- **Procedure:** Construct a model containing
  `part foo :> NonexistentTarget;`. Invoke
  `sysml-validate validate <model>`.
- **Expected:** Exit code 1; output contains a `SYSML210` diagnostic.

## Test Case TC-004 (verifies TOR-FR-004)
- **Procedure:** Construct two files, file A: `part def A :> B;` and
  file B: `part def B :> A;`. Invoke `sysml-validate validate <A>
  <B>`.
- **Expected:** Exit code 1; output contains a `SYSML220` diagnostic
  with cycle path `A :> B :> A` (or rotation).

## Test Case TC-005 (verifies TOR-FR-005)
- **Procedure:** Invoke `sysml-validate validate <model> --format json`.
- **Expected:** Output JSON contains a `metadata` object with `tool`,
  `rule_catalog`, `invocation.timestamp_utc`, `invocation.backend`,
  `invocation.strict`, `invocation.fail_on_warning`.

## Test Case TC-006 (verifies TOR-FR-006)
- **Procedure:** Invoke `sysml-validate validate <model>` under
  strace / Process Monitor with all network syscalls traced.
- **Expected:** Zero network-related syscalls.

## Test Case TC-007 (verifies TOR-FR-007)
- **Procedure:** Invoke `sysml-validate validate <model> --format sarif`.
  Validate the output against the OASIS SARIF 2.1.0 JSON Schema.
- **Expected:** Output validates; every `result` has a
  `partialFingerprints.diagnosticHash/v1` field. Two runs on the
  same input produce identical fingerprints.

## Test Case TC-008 (verifies TOR-FR-008)
- **Procedure:** Invoke `sysml-validate validate <model> --ci
  --baseline <prior.sarif> --update-baseline`. Then invoke
  `sysml-validate validate <model> --ci --baseline <prior.sarif>`
  (no `--update-baseline`).
- **Expected:** Second invocation exit code 0; results all carry
  `baselineState: "unchanged"`.

`<Add project-specific test cases.>`

## Procedure for executing the cases

Each case is executed manually or in a project CI job. Capture:
- Input model
- Full command line
- Standard output and standard error
- Exit code
- Tool version, rule catalog version, embedded library tag (from
  metadata block or `library-info`)

Record results in [`TVCR-template.md`](TVCR-template.md).
