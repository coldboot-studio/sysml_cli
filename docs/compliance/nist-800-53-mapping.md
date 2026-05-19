# NIST SP 800-53 Rev 5 Control Mapping

A `sysml-validate` deployment helps satisfy the following NIST 800-53
Rev 5 controls. The tool itself is not a system under ATO; it is a
component within the surrounding system's boundary, and this document
articulates which controls it materially supports.

## Controls supported

| Control | Title | How `sysml-validate` participates |
|---|---|---|
| **SA-11** | Developer Security Testing and Evaluation | Provides automated static analysis of SysML v2 / KerML models. Findings are emitted as SARIF 2.1.0 for ingestion by DevSecOps pipelines (Iron Bank, Platform One, GitHub Advanced Security in GovCloud, SonarQube, Azure DevOps). Failed validations gate CI per the configured rule severity. |
| **SA-15** | Development Process, Standards, and Tools | The validator enforces SysML v2 standards (OMG `formal/26-03-02`) and project-defined rules via [`sysml-validate.toml`](../../README.md#configuration). Rule severity is configurable per-project; suppressions are auditable in source. |
| **SI-7** | Software, Firmware, and Information Integrity | Every release artifact carries: SHA-256 checksum, Sigstore (Rekor-logged) signature, GPG detached signature, and SLSA v1.0 in-toto provenance attestation. See [`SECURITY.md`](../SECURITY.md). |
| **SR-3** | Supply Chain Controls and Processes | Documented vulnerability disclosure policy ([`SECURITY.md`](../SECURITY.md)); cargo-audit on every PR; Dependabot weekly cargo + actions updates; SBOM published per release. |
| **SR-4** | Provenance | SLSA v1.0 Build Level 3 in-toto attestation per release names the source commit, workflow file, runner, and build inputs. |
| **SR-11** | Component Authenticity | Dual signing (Sigstore + GPG) plus Rekor public transparency log enables consumers to detect substitution attacks. |
| **CM-7** | Least Functionality | The validator's `validate` subcommand makes **zero network calls**, has no telemetry, and does not auto-update ([`OFFLINE.md`](../OFFLINE.md)). The `--official-command` path uses positional argv with no shell process spawned, eliminating shell-injection surface. |
| **AU-2** | Event Logging | Every run emits a per-run metadata block with tool name, version, rule catalog version, RFC 3339 timestamp, backend identity, ruleset flags, config path, and baseline path. Output formats (SARIF, JUnit XML, JSON) are deterministic and machine-parseable for audit retention. |
| **AU-12** | Audit Record Generation | SARIF results carry stable `partialFingerprints.diagnosticHash/v1` values derived from `(rule code, normalized path, genericized message)`. Two runs on identical input produce identical fingerprints, enabling correlation and deduplication across the audit trail. |

## Controls partially supported (deployment-dependent)

The following controls depend on the surrounding system's deployment
configuration, but `sysml-validate` does not impede them and provides
hooks where relevant:

| Control | Title | How the tool accommodates |
|---|---|---|
| **AC-3** | Access Enforcement | The tool reads only files passed on the CLI and writes only to stdout / the `--baseline` path / files the official backend chooses. Honors the host filesystem ACLs. |
| **AC-6** | Least Privilege | No special privileges required to run. Executes as the invoking user. |
| **CM-3** | Configuration Change Control | Rule catalog version (currently 0.5.0) is bumped whenever rule meanings change. Consumers can gate baselines on this version. |
| **CM-6** | Configuration Settings | `sysml-validate.toml` provides the deployment-time configuration surface; `deny_unknown_fields` rejects typos at parse time. |
| **CP-9** | System Backup | Output to stdout is captured by the calling pipeline; baseline files are caller-managed. |
| **IA-2** | Identification and Authentication | Tool inherits the invoking shell's identity. Does not perform its own authentication. |

## Controls explicitly out of scope

The following 800-53 controls are not in this tool's scope. A
deployment depending on them must satisfy them via the surrounding
system.

- **AC-2** Account Management — the tool has no user accounts
- **AT-2/3** Awareness Training — N/A to an automated tool
- **CA-\*** Assessment and Authorization — system-level, not tool-level
- **CP-2/3/4** Contingency Planning — N/A to a CLI utility
- **IR-\*** Incident Response — except RV.\* via [`SECURITY.md`](../SECURITY.md)
- **PE-\*** Physical and Environmental — N/A to software
- **PS-\*** Personnel Security — N/A

## DoD Cloud SRG impact levels

The validator is built to run cleanly in any impact level:

- **IL2** (public/non-CUI): trivially compatible.
- **IL4** (CUI): compatible. Tool is offline-capable; no telemetry; no
  auto-update. `cargo vendor` recipe documented in [`OFFLINE.md`](../OFFLINE.md)
  for fully air-gapped builds.
- **IL5** (CUI national-security): same as IL4. FIPS 140-3
  considerations: the tool currently performs no cryptographic
  operations at runtime, so FIPS does not apply. A `--fips` build
  flag is reserved ([PRD US-307](../PRD-government-readiness.md)) for
  the day signing operations are added inline.

## CMMC 2.0 Level 2 cross-reference

CMMC L2 mirrors NIST SP 800-171 Rev 2. See
[`cmmc-l2-deployment.md`](cmmc-l2-deployment.md) for the contractor-
side deployment recipe.
