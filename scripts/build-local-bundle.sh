#!/usr/bin/env bash
#
# Build the end-user release bundle locally so a developer can see
# exactly what ships to end users without pushing a tag.
#
# Produces dist/bundle/sysml-validate-<version>-<target>.tar.gz with
# the same internal layout as the CI-produced release bundle, MINUS
# the cryptographic trust artifacts that require GitHub OIDC and a
# published GPG key:
#   - cosign signature        — skipped (CI requires OIDC token)
#   - GPG signature           — skipped (requires private key)
#   - SLSA in-toto provenance — skipped (requires actions/attest-build-provenance)
#
# What IS included (everything else a real release ships):
#   - the compiled binary (release profile)
#   - SHA-256 checksum
#   - CycloneDX 1.6 SBOM        (if cargo-cyclonedx is installed)
#   - SPDX 3.0 SBOM             (if syft is installed)
#   - every doc:
#       EXECUTIVE_SUMMARY, TECH_MANUAL, PRD, SECURITY, OFFLINE,
#       THREAT_MODEL, REPRODUCING, accessibility,
#       differential-corpus-report, compliance/* (full pack incl.
#       DO-330 kit + NPR 7150.2D + SSDF + 800-53 + CMMC L2 + INDEX)
#   - the agentskills.io agent skill (skills/sysml-validate/)
#   - top-level: VERSION, LICENSE, NOTICE.md, README.md
#   - BUNDLE-MANIFEST.txt with SHA-256 of every file
#
# Usage:
#   scripts/build-local-bundle.sh                  # auto-detect target
#   scripts/build-local-bundle.sh <target-triple>  # explicit target

set -euo pipefail

# ---------------------------------------------------------------------
# Resolve target triple
# ---------------------------------------------------------------------

TARGET="${1:-}"
if [[ -z "${TARGET}" ]]; then
  # rustc -Vv prints `host: <triple>` — that's the native target.
  TARGET="$(rustc -Vv | awk '/^host:/ {print $2}')"
  if [[ -z "${TARGET}" ]]; then
    echo "error: could not auto-detect target triple; pass it as arg 1" >&2
    exit 2
  fi
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "${REPO_ROOT}"

VERSION="$(awk -F\" '/^version =/ {print $2; exit}' Cargo.toml)"
if [[ -z "${VERSION}" ]]; then
  echo "error: could not read version from Cargo.toml" >&2
  exit 2
fi

case "${TARGET}" in
  *windows*) EXT=".exe" ;;
  *)         EXT=""     ;;
esac

DIST_DIR="dist/local"
BIN_NAME="sysml-validate-${TARGET}${EXT}"

echo "==> Building sysml-validate ${VERSION} for ${TARGET}"
echo "    Output will be dist/bundle/sysml-validate-${VERSION}-${TARGET}.tar.gz"
echo

# ---------------------------------------------------------------------
# Build the binary
# ---------------------------------------------------------------------

cargo build --release --locked

rm -rf "${DIST_DIR}"
mkdir -p "${DIST_DIR}"

cp "target/release/sysml-validate${EXT}" "${DIST_DIR}/${BIN_NAME}"

# ---------------------------------------------------------------------
# SHA-256 (real)
# ---------------------------------------------------------------------

if command -v sha256sum >/dev/null 2>&1; then
  SHA_CMD=(sha256sum)
else
  SHA_CMD=(shasum -a 256)
fi

(
  cd "${DIST_DIR}"
  "${SHA_CMD[@]}" "${BIN_NAME}" > "${BIN_NAME}.sha256"
)

# ---------------------------------------------------------------------
# Placeholder signatures so the bundle layout is identical to CI's
# ---------------------------------------------------------------------

cat > "${DIST_DIR}/${BIN_NAME}.cosign.bundle" <<EOF
# LOCAL BUILD — NOT A REAL COSIGN SIGNATURE
#
# This file is a placeholder. A real cosign.bundle is produced by
# .github/workflows/release.yml on a GitHub-hosted runner using
# Sigstore keyless OIDC. Local builds cannot produce one because
# they do not have an OIDC identity for Sigstore to issue a
# certificate against.
#
# To get a real signature, push a v* tag and download the release
# asset.
EOF

# ---------------------------------------------------------------------
# SBOMs (real if tools installed, placeholders otherwise)
# ---------------------------------------------------------------------

CDX_FILE="${DIST_DIR}/sysml-validate-${TARGET}.cdx.json"
SPDX_FILE="${DIST_DIR}/sysml-validate-${TARGET}.spdx.json"

if command -v cargo-cyclonedx >/dev/null 2>&1; then
  echo "==> Generating CycloneDX 1.6 SBOM"
  cargo cyclonedx --format json --spec-version 1.6 \
    --override-filename sysml-validate.cdx >/dev/null 2>&1 || true
  if [[ -f sysml-validate.cdx.json ]]; then
    mv sysml-validate.cdx.json "${CDX_FILE}"
  fi
else
  cat > "${CDX_FILE}" <<EOF
{"_local_placeholder": "install cargo-cyclonedx for real CycloneDX 1.6 SBOM"}
EOF
fi

if command -v syft >/dev/null 2>&1; then
  echo "==> Generating SPDX 3.0 SBOM"
  syft scan . -o spdx-json="${SPDX_FILE}" >/dev/null 2>&1 || true
else
  cat > "${SPDX_FILE}" <<EOF
{"_local_placeholder": "install syft for real SPDX 3.0 SBOM"}
EOF
fi

# ---------------------------------------------------------------------
# Hand off to the canonical assembler (same path CI takes)
# ---------------------------------------------------------------------

# Force tar.gz output even on Windows for local builds — the structure
# is the same; Windows users opening it can use 7-Zip / built-in tar.
# (The CI script branches to .zip for Windows because that's what
# Windows release consumers expect; here we just want to inspect the
# layout and the user may not have `zip` on PATH.)
echo "==> Assembling bundle"
BUNDLE_FORCE_ARCHIVE_KIND=tgz \
  bash scripts/assemble-release-bundle.sh "${VERSION}" "${TARGET}" "${DIST_DIR}"

ARCHIVE="dist/local/bundle/sysml-validate-${VERSION}-${TARGET}.tar.gz"
if [[ ! -f "${ARCHIVE}" ]]; then
  # On Windows targets the assembler writes .zip. The local script
  # asks the assembler to use the target as given, so for *-windows-*
  # we get .zip. Surface whichever was produced.
  ARCHIVE="$(ls "${DIST_DIR}/bundle/"*"${VERSION}-${TARGET}".* 2>/dev/null | head -n 1 || true)"
fi

echo
echo "==> Local bundle ready: ${ARCHIVE}"
echo
echo "Inspect contents:"
case "${ARCHIVE}" in
  *.tar.gz) echo "  tar -tzf ${ARCHIVE} | head -40" ;;
  *.zip)    echo "  unzip -l ${ARCHIVE} | head -40" ;;
esac
echo
echo "NOTE: this is a LOCAL bundle. It is missing:"
echo "  - the real cosign signature (CI-only, requires GitHub OIDC)"
echo "  - the GPG signature (requires the project signing key)"
echo "  - the SLSA Build L3 provenance attestation"
echo
echo "For the real signed-and-attested deliverable, pull a tagged"
echo "release from GitHub."
