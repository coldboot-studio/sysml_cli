# NIST SP 800-218 Secure Software Development Framework (SSDF) Mapping

This document maps each NIST SP 800-218 v1.1 practice to the
corresponding evidence in this repository. It is intended as the
"SSDF self-attestation" reference a defense prime or federal civilian
SCRM reviewer would consult.

The four SSDF practice groups are **PO** (Prepare the Organization),
**PS** (Protect the Software), **PW** (Produce Well-Secured Software),
and **RV** (Respond to Vulnerabilities). We address the subset most
relevant to an OSS Rust CLI; controls covering organizational HR,
physical security, etc. are not applicable.

OMB M-26-05 (Jan 23, 2026) rescinded the universal CISA Common Form
self-attestation mandate; agencies now run risk-based vendor
assurance. Primes still ask for SSDF mappings, so this document is
maintained.

## PO — Prepare the Organization

| Practice | What it requires | Evidence |
|---|---|---|
| PO.1 (Define security requirements for software development) | Documented security requirements at the project level. | [`docs/SECURITY.md`](../SECURITY.md) — vulnerability policy, supported versions, verification recipes. [`docs/THREAT_MODEL.md`](../THREAT_MODEL.md) — trust boundaries enumerated. |
| PO.2 (Implement roles and responsibilities) | Defined responsibilities for security. | Maintainer is responsible for security review; private-advisory channel routes reports through GitHub. |
| PO.3 (Implement supporting toolchains) | Use tools that enforce security practices. | [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) runs cargo-audit on every PR; cargo-cyclonedx + syft generate SBOMs on release. |
| PO.4 (Define criteria for software security checks) | Document required security checks. | This document; [`docs/PRD-government-readiness.md`](../PRD-government-readiness.md) §3 enumerates them. |
| PO.5 (Implement and maintain secure environments) | Hardened development infrastructure. | GitHub-hosted runners with documented base images; release jobs use `permissions: contents: read` and explicit `id-token: write` only when needed. |

## PS — Protect the Software

| Practice | What it requires | Evidence |
|---|---|---|
| PS.1 (Protect all forms of code from unauthorized access and tampering) | Branch protection, signed commits, code reviews. | Repository branch protection on `main`; releases are tagged. The release workflow signs artifacts with Sigstore (Rekor-logged keyless) AND GPG. |
| PS.2 (Provide a mechanism for verifying software release integrity) | Consumers can verify integrity. | SHA-256 checksums + Sigstore cosign bundles + GPG `.asc` + SLSA v1.0 in-toto provenance, all attached to every GitHub Release. Verification recipe in [`docs/SECURITY.md`](../SECURITY.md). |
| PS.3 (Archive and protect each software release) | Releases are retained and tamper-evident. | GitHub Releases retain artifacts indefinitely. SLSA provenance is logged in Sigstore Rekor (public, immutable transparency log). |

## PW — Produce Well-Secured Software

| Practice | What it requires | Evidence |
|---|---|---|
| PW.1 (Design software to meet security requirements and mitigate security risks) | Documented threat model. | [`docs/THREAT_MODEL.md`](../THREAT_MODEL.md) enumerates 5 trust boundaries with mitigations + explicit out-of-scope items. |
| PW.2 (Review the software design to verify compliance) | Security review of design. | PR-time code review; CI runs clippy with `-D warnings`. |
| PW.4 (Reuse existing, well-secured software when feasible) | Vendor-pinning of dependencies. | `Cargo.lock` committed. Dependabot weekly cargo + actions updates ([`.github/dependabot.yml`](../../.github/dependabot.yml)). Cargo-audit blocks PRs with known advisories. |
| PW.5 (Create source code by adhering to secure coding practices) | Static analysis. | clippy at error level; Rust's borrow checker; no `unsafe` blocks (verified via grep). |
| PW.6 (Configure the compilation, interpreter, and build processes to improve executable security) | Hardened build flags. | [`Cargo.toml`](../../Cargo.toml) `[profile.release]`: `codegen-units = 1`, `lto = "fat"`, `strip = "symbols"`, `panic = "abort"`. `--remap-path-prefix` in [`.cargo/config.toml`](../../.cargo/config.toml). |
| PW.7 (Review and/or analyze human-readable code) | Code review before merge. | GitHub branch protection requires PR review on `main`. |
| PW.8 (Test executable code) | Functional + security testing. | 116 unit tests + 2 differential corpus tests. cargo-audit on every PR. |
| PW.9 (Configure software to have secure settings by default) | Secure defaults. | Validator makes zero network calls by default ([`docs/OFFLINE.md`](../OFFLINE.md)); no telemetry; `--official-command` uses positional argv, no shell. |

## RV — Respond to Vulnerabilities

| Practice | What it requires | Evidence |
|---|---|---|
| RV.1 (Identify and confirm vulnerabilities on an ongoing basis) | Vulnerability monitoring. | cargo-audit in CI against the RustSec advisory database; Dependabot security alerts. |
| RV.2 (Assess, prioritize, and remediate vulnerabilities) | Documented response process. | [`docs/SECURITY.md`](../SECURITY.md) commits to 3-business-day acknowledgment, 10-business-day triage. |
| RV.3 (Analyze vulnerabilities to identify their root causes) | Root-cause analysis. | Each security fix lands as a discrete commit referencing the advisory; release notes describe the issue. |

## Quick reference: artifacts produced per release

For each tagged release, the [release workflow](../../.github/workflows/release.yml) produces:

- The compiled binary
- SHA-256 checksum
- CycloneDX 1.6 SBOM (`*.cdx.json`)
- SPDX 3.0 SBOM (`*.spdx.json`)
- Sigstore cosign bundle (`*.cosign.bundle`)
- GPG detached signature (`*.asc`)
- SLSA v1.0 in-toto provenance (via `actions/attest-build-provenance`)

This satisfies PS.1, PS.2, PS.3, PW.6, and provides the supply-chain
evidence reviewers typically request.

## Related

- [`SECURITY.md`](../SECURITY.md) — verification recipes for the above artifacts
- [`nist-800-53-mapping.md`](nist-800-53-mapping.md) — NIST 800-53 Rev 5 control mapping
- [`cmmc-l2-deployment.md`](cmmc-l2-deployment.md) — CMMC Level 2 deployment recipe
- [`do-330-qualification-kit/`](do-330-qualification-kit/) — DO-330 TQL-5 templates
