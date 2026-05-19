# Security Policy

## Reporting a vulnerability

**Please do not file public GitHub issues for security vulnerabilities.**

Report by one of:

1. **GitHub private security advisories** — the preferred channel.
   `Security` tab → `Report a vulnerability`.
2. **Email** — the maintainers' security contact listed on the
   repository profile page. PGP encryption appreciated; key fingerprint
   appears below.

We commit to:

- acknowledging your report within **3 business days**,
- providing an initial triage assessment within **10 business days**,
- coordinating a fix and disclosure timeline with you,
- crediting your finding in the release notes (unless you prefer
  anonymity).

This policy maps to NIST SP 800-218 SSDF practices **RV.1** (identify
vulnerabilities), **RV.2** (analyze and respond), and **RV.3**
(communicate root cause and remediation).

## Supported versions

| Version | Supported       |
|---------|-----------------|
| 0.4.x   | yes             |
| < 0.4   | no — please upgrade |

## Verifying a release

Every release artifact in [GitHub Releases](../../releases) ships with
multiple integrity signals. **Verify the binary before running it in any
trust-sensitive environment.**

### 1. SHA-256

```bash
sha256sum -c sysml-validate-<target>.sha256
```

### 2. Sigstore / cosign signature (keyless OIDC)

Recommended path for most consumers. The signature is logged in the
Sigstore Rekor transparency log, so any unauthorized signing event is
publicly visible.

```bash
# Install cosign (https://docs.sigstore.dev/cosign/system_config/installation/).
cosign verify-blob \
  --bundle sysml-validate-<target>.cosign.bundle \
  --certificate-identity-regexp "^https://github.com/<owner>/<repo>/" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
  sysml-validate-<target>
```

The `--certificate-identity-regexp` should pin to **your** fork of the
repository if you build internally; the canonical upstream pattern will
be published once the first release ships.

### 3. GPG detached signature

For air-gapped environments and consumers that prefer the traditional
OpenPGP trust path.

```bash
# Fetch the project signing key once. The fingerprint is published on
# the project landing page and at keys.openpgp.org.
gpg --keyserver keys.openpgp.org \
    --recv-keys <FINGERPRINT_TO_BE_PUBLISHED_AT_FIRST_RELEASE>

gpg --verify sysml-validate-<target>.asc sysml-validate-<target>
```

### 4. SLSA v1.0 Build Level 3 provenance

The release workflow attaches an in-toto attestation that names the
exact source commit, the workflow file, and the runner used. Verify
that the binary you have was built from the source you expect:

```bash
gh attestation verify sysml-validate-<target> \
  --owner <owner>
```

See `actions/attest-build-provenance` documentation for the full
verification recipe.

### 5. SBOM

CycloneDX 1.6 (`*.cdx.json`) and SPDX 3.0 (`*.spdx.json`) SBOMs are
attached to every release. They enumerate every component in the build
graph. Feed them to your dependency-scanning tool of choice (Grype,
Trivy, Dependency-Track, etc.) to gate on advisories.

## Threat model

See [`THREAT_MODEL.md`](THREAT_MODEL.md) for the trust boundaries this
tool defends and the boundaries it explicitly does not.

## Air-gap and offline deployment

See [`OFFLINE.md`](OFFLINE.md) for the air-gap contract: which
subcommands touch the network (none today), which environment variables
are honored, and what `--offline` deployments look like.

## NIST 800-53 cross-references

The most relevant controls a sysml-validate deployment helps satisfy:

| Control | Practice |
|---------|---------|
| SA-11   | Developer security testing — validator gates on commits |
| SA-15   | Development process / standards / tools |
| SI-7    | Software / firmware integrity — signed releases + SBOM |
| SR-3    | Supply chain controls — SBOM + provenance |
| SR-4    | Provenance — SLSA v1.0 in-toto attestation |
| SR-11   | Component authenticity — Sigstore + GPG signatures |
| CM-7    | Least functionality — no network in `validate` |
| AU-2/12 | Auditable events — per-run metadata block + SARIF |

Full control-by-control mapping:
[`docs/compliance/nist-800-53-mapping.md`](compliance/nist-800-53-mapping.md).
NIST SSDF mapping: [`docs/compliance/ssdf-mapping.md`](compliance/ssdf-mapping.md).
CMMC L2 deployment recipe:
[`docs/compliance/cmmc-l2-deployment.md`](compliance/cmmc-l2-deployment.md).
DO-330 TQL-5 qualification kit skeleton:
[`docs/compliance/do-330-qualification-kit/`](compliance/do-330-qualification-kit/).
NASA NPR 7150.2D tool validation template:
[`docs/compliance/nasa-npr-7150-2d-tool-validation.md`](compliance/nasa-npr-7150-2d-tool-validation.md).
