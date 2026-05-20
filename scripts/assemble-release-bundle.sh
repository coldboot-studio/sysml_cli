#!/usr/bin/env bash
#
# Assemble a government-ready release bundle for one target triple.
#
# Inputs: a `dist/` directory pre-populated by the matrix-build job of
# .github/workflows/release.yml with:
#   - sysml-validate-<target>(.exe)
#   - sysml-validate-<target>(.exe).sha256
#   - sysml-validate-<target>(.exe).cosign.bundle
#   - sysml-validate-<target>(.exe).asc          (optional)
#   - sysml-validate-<target>.cdx.json
#   - sysml-validate-<target>.spdx.json
#   - provenance.intoto.jsonl                    (optional, from attest-build-provenance)
#
# Output: a single archive at
#   dist/bundle/sysml-validate-<version>-<target>.tar.gz   (Unix-like targets)
#   dist/bundle/sysml-validate-<version>-<target>.zip      (Windows targets)
#
# The bundle is structured per docs/compliance/INDEX.md §"File inventory."
# A BUNDLE-MANIFEST.txt at the bundle root lists every file with its
# SHA-256 so the whole tree can be verified at once with
#   sha256sum -c BUNDLE-MANIFEST.txt
#
# This script is deterministic: file ordering is sorted, the manifest is
# generated last, and tar/zip are invoked with sort flags so a bit-
# identical bundle results from a bit-identical input dist/.

set -euo pipefail

# ---------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------

VERSION="${1:-}"
TARGET="${2:-}"
DIST_DIR="${3:-dist}"

if [[ -z "$VERSION" || -z "$TARGET" ]]; then
  echo "usage: $0 <version> <target-triple> [dist-dir]" >&2
  echo "example: $0 0.15.0 x86_64-unknown-linux-gnu dist" >&2
  exit 2
fi

case "$TARGET" in
  *windows*) EXT=".exe"; ARCHIVE_KIND="zip"  ;;
  *)         EXT="";     ARCHIVE_KIND="tgz"  ;;
esac

REPO_ROOT="$(git rev-parse --show-toplevel)"
BUNDLE_NAME="sysml-validate-${VERSION}-${TARGET}"
STAGE_DIR="${DIST_DIR}/bundle/${BUNDLE_NAME}"
ARCHIVE_DIR="${DIST_DIR}/bundle"

rm -rf "${STAGE_DIR}"
mkdir -p \
  "${STAGE_DIR}/bin" \
  "${STAGE_DIR}/sbom" \
  "${STAGE_DIR}/attestation" \
  "${STAGE_DIR}/docs/compliance/do-330-qualification-kit" \
  "${STAGE_DIR}/skills"

# ---------------------------------------------------------------------
# Stage binary + signatures
# ---------------------------------------------------------------------

BIN_NAME="sysml-validate-${TARGET}${EXT}"
cp "${DIST_DIR}/${BIN_NAME}"                    "${STAGE_DIR}/bin/sysml-validate${EXT}"
cp "${DIST_DIR}/${BIN_NAME}.sha256"             "${STAGE_DIR}/bin/sysml-validate${EXT}.sha256"
cp "${DIST_DIR}/${BIN_NAME}.cosign.bundle"      "${STAGE_DIR}/bin/sysml-validate${EXT}.cosign.bundle"
if [[ -f "${DIST_DIR}/${BIN_NAME}.asc" ]]; then
  cp "${DIST_DIR}/${BIN_NAME}.asc"              "${STAGE_DIR}/bin/sysml-validate${EXT}.asc"
fi

# ---------------------------------------------------------------------
# Stage SBOMs
# ---------------------------------------------------------------------

cp "${DIST_DIR}/sysml-validate-${TARGET}.cdx.json"  "${STAGE_DIR}/sbom/sysml-validate.cdx.json"
cp "${DIST_DIR}/sysml-validate-${TARGET}.spdx.json" "${STAGE_DIR}/sbom/sysml-validate.spdx.json"

# ---------------------------------------------------------------------
# Stage SLSA provenance, if present
# ---------------------------------------------------------------------

if [[ -f "${DIST_DIR}/provenance.intoto.jsonl" ]]; then
  cp "${DIST_DIR}/provenance.intoto.jsonl" "${STAGE_DIR}/attestation/provenance.intoto.jsonl"
fi

# ---------------------------------------------------------------------
# Stage top-level files
# ---------------------------------------------------------------------

echo "${VERSION}" > "${STAGE_DIR}/VERSION"
cp "${REPO_ROOT}/LICENSE"   "${STAGE_DIR}/LICENSE"     2>/dev/null || true
cp "${REPO_ROOT}/NOTICE.md" "${STAGE_DIR}/NOTICE.md"   2>/dev/null || true
cp "${REPO_ROOT}/README.md" "${STAGE_DIR}/README.md"   2>/dev/null || true
cp "${REPO_ROOT}/CHANGELOG.md" "${STAGE_DIR}/CHANGELOG.md" 2>/dev/null || true

# ---------------------------------------------------------------------
# Stage docs (every doc the conformance index references)
# ---------------------------------------------------------------------

DOCS_SRC="${REPO_ROOT}/docs"
DOCS_DST="${STAGE_DIR}/docs"

for doc in \
  EXECUTIVE_SUMMARY.md \
  TECH_MANUAL.md \
  PRD-government-readiness.md \
  SECURITY.md \
  OFFLINE.md \
  THREAT_MODEL.md \
  REPRODUCING.md \
  accessibility.md \
  differential-corpus-report.md
do
  if [[ -f "${DOCS_SRC}/${doc}" ]]; then
    cp "${DOCS_SRC}/${doc}" "${DOCS_DST}/${doc}"
  fi
done

for doc in \
  INDEX.md \
  ssdf-mapping.md \
  nist-800-53-mapping.md \
  cmmc-l2-deployment.md \
  nasa-npr-7150-2d-tool-validation.md
do
  if [[ -f "${DOCS_SRC}/compliance/${doc}" ]]; then
    cp "${DOCS_SRC}/compliance/${doc}" "${DOCS_DST}/compliance/${doc}"
  fi
done

if [[ -d "${DOCS_SRC}/compliance/do-330-qualification-kit" ]]; then
  cp -r "${DOCS_SRC}/compliance/do-330-qualification-kit/." \
        "${DOCS_DST}/compliance/do-330-qualification-kit/"
fi

# ---------------------------------------------------------------------
# Stage agent skill(s) (US-311 / Batch Q)
#
# Per agentskills.io: a skill is a directory containing SKILL.md.
# Copy the whole tree so the consumer can drop skills/sysml-validate/
# into their agent's skills directory verbatim.
# ---------------------------------------------------------------------

SKILLS_SRC="${REPO_ROOT}/skills"
if [[ -d "${SKILLS_SRC}" ]]; then
  cp -r "${SKILLS_SRC}/." "${STAGE_DIR}/skills/"
fi

# ---------------------------------------------------------------------
# Generate BUNDLE-MANIFEST.txt (sorted, deterministic)
# ---------------------------------------------------------------------

# Pick the right SHA-256 tool per host.
if command -v sha256sum >/dev/null 2>&1; then
  SHA_CMD=(sha256sum)
else
  SHA_CMD=(shasum -a 256)
fi

(
  cd "${STAGE_DIR}"
  # `find -print0 | sort -z` gives stable cross-host ordering. Exclude
  # the manifest itself from its own hash.
  find . -type f ! -name BUNDLE-MANIFEST.txt -print0 |
    LC_ALL=C sort -z |
    xargs -0 "${SHA_CMD[@]}" > BUNDLE-MANIFEST.txt
)

# ---------------------------------------------------------------------
# Pack the archive
# ---------------------------------------------------------------------

mkdir -p "${ARCHIVE_DIR}"

case "${ARCHIVE_KIND}" in
  tgz)
    # GNU tar's --sort=name and pinning ownership / timestamp gives a
    # deterministic archive given a deterministic tree.
    tar \
      --sort=name \
      --owner=0 --group=0 --numeric-owner \
      --mtime="@${SOURCE_DATE_EPOCH:-0}" \
      -czf "${ARCHIVE_DIR}/${BUNDLE_NAME}.tar.gz" \
      -C "${DIST_DIR}/bundle" "${BUNDLE_NAME}"
    echo "Bundle: ${ARCHIVE_DIR}/${BUNDLE_NAME}.tar.gz"
    ;;
  zip)
    # `zip -X` strips extra fields that vary across hosts; `-r` recursive.
    # Path arg is relative so the archive's internal paths are stable.
    (
      cd "${DIST_DIR}/bundle"
      zip -X -r "${BUNDLE_NAME}.zip" "${BUNDLE_NAME}" >/dev/null
    )
    mv "${DIST_DIR}/bundle/${BUNDLE_NAME}.zip" "${ARCHIVE_DIR}/${BUNDLE_NAME}.zip"
    echo "Bundle: ${ARCHIVE_DIR}/${BUNDLE_NAME}.zip"
    ;;
esac

# Final SHA-256 of the bundle archive itself, for the release notes /
# advertised hash.
(
  cd "${ARCHIVE_DIR}"
  "${SHA_CMD[@]}" "${BUNDLE_NAME}.${ARCHIVE_KIND/tgz/tar.gz}" \
    > "${BUNDLE_NAME}.${ARCHIVE_KIND/tgz/tar.gz}.sha256"
)

echo "OK"
