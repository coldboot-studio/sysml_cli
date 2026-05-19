# Tool Configuration Management Plan — TEMPLATE OUTLINE

> Project-specific completion required.

## 1. Configuration Items

| CI | Identification | Verification |
|---|---|---|
| Tool binary | SHA-256 from published checksum | `sha256sum -c sysml-validate-<target>.sha256` |
| Tool version | Cargo.toml `version` | `sysml-validate --version` |
| Rule catalog | `src/report.rs` `RULE_CATALOG_VERSION` | Metadata block, JSON/SARIF field `rule_catalog.version` |
| Embedded library | Submodule pin in `vendor/sysml-v2-release` | `sysml-validate library-info` reports the release tag |
| Configuration file | `sysml-validate.toml` if used | Hash and store with project artifacts |

## 2. Baseline Definition
- The qualified baseline is the set of configuration items captured at
  the point of TVCR sign-off, recorded by hash in the TAS.

## 3. Change Control
- Tool version bump requires re-qualification.
- Rule catalog bump requires re-qualification (verifies all TOR-FR
  requirements still pass).
- Embedded library tag bump requires re-qualification.
- Project-specific configuration file changes require updating the
  baseline hash; re-qualification not required if the same rule set
  remains in effect.

## 4. Reproducibility
- See [`../../REPRODUCING.md`](../../REPRODUCING.md). An independent
  byte-for-byte rebuild of the binary from source validates the
  configuration item.

## 5. Provenance
- SLSA v1.0 in-toto attestations are part of the CM record.
- Sigstore Rekor transparency-log entries are part of the CM record.

## 6. Archival
- Each release is archived in GitHub Releases. Adopting projects
  should mirror the binary, SBOMs, signatures, and provenance into
  their own controlled storage.
