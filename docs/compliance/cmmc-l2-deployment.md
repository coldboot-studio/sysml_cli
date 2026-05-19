# CMMC 2.0 Level 2 Deployment Guide

This document describes how to deploy `sysml-validate` inside a
contractor environment that must demonstrate CMMC 2.0 Level 2
compliance (110 controls drawn from NIST SP 800-171 Rev 2, under
DFARS 252.204-7012).

The tool itself is not a CMMC boundary. It is software used inside
your boundary. This guide ensures its deployment does not undermine
your existing controls.

## Pre-deployment verification

Before deploying, verify the release as documented in
[`SECURITY.md`](../SECURITY.md):

1. SHA-256 against published checksum.
2. Sigstore cosign signature (Rekor-logged, public transparency log).
3. GPG detached signature against the published project key fingerprint.
4. SLSA v1.0 build provenance (`gh attestation verify`).
5. Review the CycloneDX 1.6 and SPDX 3.0 SBOMs against your
   organization's dependency-policy list.

Skipping any of these defeats the supply-chain protections the
release pipeline provides.

## Deployment model

Recommended: install the binary in a read-only system path
(e.g., `/usr/local/bin/sysml-validate` on Linux,
`%ProgramFiles%\sysml-validate\sysml-validate.exe` on Windows) with
execute permission for the intended users only. The binary requires
no companion files at runtime (the SysML v2 standard library is
embedded).

The binary is self-contained:

- No registry entries
- No persistent state directory
- No log files (output captured by the caller)
- No background services
- No outbound network connections by default

## CMMC L2 control mapping (contractor-side)

| Domain | Control | How deployment satisfies |
|---|---|---|
| **AC** Access Control | AC.L2-3.1.1 (Limit information system access to authorized users) | Tool inherits the host OS's filesystem ACLs. Install with execute permission limited to your modeling users. |
| | AC.L2-3.1.20 (Verify and control connections to and use of external systems) | Default mode makes zero network calls. `--official-command` invokes a user-specified child binary; that child binary's connections are the contractor's responsibility to audit. |
| **AU** Audit and Accountability | AU.L2-3.3.1 (Create and retain system audit records) | Pipe SARIF or JSON output to your audit retention store. Per-run metadata block includes the auditable identity of the run. |
| | AU.L2-3.3.2 (Ensure that the actions of individual users can be uniquely traced) | Run identity is the invoking user's OS identity. Capture `$USER` / `%USERNAME%` alongside the validator output. |
| **CM** Configuration Management | CM.L2-3.4.1 (Establish and maintain baseline configurations) | Use the `--baseline` flag to record an accepted-findings baseline and gate CI on drift only. |
| | CM.L2-3.4.2 (Establish and enforce security configuration settings) | `sysml-validate.toml` rejects unknown fields (`deny_unknown_fields`) — typos surface at parse time, not at silent default. |
| **IA** Identification and Authentication | IA.L2-3.5.1 (Identify information system users) | Tool inherits OS identity. Does not perform its own auth. |
| **SI** System and Information Integrity | SI.L2-3.14.1 (Identify, report, and correct system flaws in a timely manner) | cargo-audit catches advisories on every PR; Dependabot opens patch PRs weekly; vulnerability disclosure policy in [`SECURITY.md`](../SECURITY.md). |
| | SI.L2-3.14.2 (Provide protection from malicious code) | Releases are signed (Sigstore + GPG) with public transparency-log proof. SHA-256 checksums catch tampering at rest. |
| | SI.L2-3.14.7 (Identify unauthorized use of organizational systems) | Output retention enables retrospective analysis. |

## Air-gapped (disconnected) deployment

`sysml-validate` is designed for IL4/IL5 enclaves. To deploy in an
air-gapped environment:

1. Transfer the verified release binary into the enclave via your
   approved file transfer process.
2. Optionally transfer the SBOM files alongside for archival.
3. The binary runs without any further network setup.

For air-gapped *builds from source*, see the `cargo vendor` recipe
in [`OFFLINE.md`](../OFFLINE.md).

## Hardened-container example

Sample Dockerfile for hardened Iron Bank-style deployment:

```dockerfile
FROM registry.access.redhat.com/ubi9-minimal:latest AS final
USER 1000:1000
COPY --chown=1000:1000 sysml-validate /usr/local/bin/sysml-validate
RUN chmod 0755 /usr/local/bin/sysml-validate
ENTRYPOINT ["/usr/local/bin/sysml-validate"]
```

This produces a container that:

- Runs as a non-root user (`1000:1000`)
- Contains only the validator binary (no shell, no package manager
  payload beyond UBI's minimal base)
- Has no writable filesystem layer beyond `/tmp`
- Exposes no ports

For Platform One Big Bang adoption, contact your platform's SAR
process; this container is a starting point, not a turnkey.

## What this document does NOT cover

This guide covers tool **deployment**. It does NOT cover:

- The contractor's broader CMMC L2 control implementation (AC, IA,
  AT, CM, etc. at the organizational level).
- Boundary definition or scoping of the CUI environment.
- The contractor's incident-response plan (IR-\* family).
- Specific approval / authorization actions (AO sign-offs, ATO
  packages).

Those remain the contractor's responsibility.

## Related

- [`SECURITY.md`](../SECURITY.md) — release verification recipe
- [`OFFLINE.md`](../OFFLINE.md) — air-gap contract and `cargo vendor`
- [`THREAT_MODEL.md`](../THREAT_MODEL.md) — trust boundaries
- [`nist-800-53-mapping.md`](nist-800-53-mapping.md) — Rev 5 mapping
- [`ssdf-mapping.md`](ssdf-mapping.md) — SSDF mapping
