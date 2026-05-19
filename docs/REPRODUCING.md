# Reproducing a `sysml-validate` Release

This document describes how to verify, byte-for-byte, that a published
`sysml-validate` binary was built from the corresponding source revision
in this repository.

Reproducible builds are a NIST SSDF PS.3 practice and an informational
goal of SLSA v1.0 Build Level 4. They make supply-chain compromise
detectable independently of any single CI provider.

## What "reproducible" means here

Given:

- a published release artifact `sysml-validate-<target>` from a GitHub
  Release,
- the git revision the release tag points to,
- **all submodules at the revisions recorded in that git revision**
  (notably [`vendor/sysml-v2-release/`](../vendor/sysml-v2-release/),
  which embeds the OMG SysML v2 standard library at release tag
  `2026-04`, commit `9baca5908ca28b53da085de69336fde48420ea8f`),
- the Rust toolchain pinned in [`rust-toolchain.toml`](../rust-toolchain.toml),
- the same build target triple,

an independent build using the recipe below MUST produce a binary with
the same SHA-256 digest as the published artifact.

Note: the library is embedded into the binary via `include_dir!` at
compile time. Two builds against different upstream library revisions
will produce different binaries even if the rest of the source is
identical. The submodule pin is part of the reproducibility contract.

## The recipe

```bash
# 1. Clone WITH submodules, then check out the release tag.
git clone --recurse-submodules https://github.com/<owner>/<repo> sysml-validate
cd sysml-validate
git checkout v0.5.0       # the tag of interest
git submodule update --init --recursive   # if --recurse-submodules was omitted

# 2. Confirm the toolchain pin. cargo will install the exact version
#    listed in rust-toolchain.toml on first invocation.
cat rust-toolchain.toml   # channel = "1.85.0", components, profile

# 3. Pin the build epoch to the tag commit's author date. This matches
#    what the release workflow does.
export SOURCE_DATE_EPOCH=$(git log -1 --pretty=%ct)

# 4. Build with the locked dependency graph. --locked forbids any
#    Cargo.lock modification; the build fails if a dep is missing or
#    different from what was published.
cargo build --release --locked --target x86_64-unknown-linux-gnu

# 5. Compare.
expected_sha=$(curl -fsSL \
  https://github.com/<owner>/<repo>/releases/download/v0.4.0/sysml-validate-x86_64-unknown-linux-gnu.sha256 \
  | awk '{print $1}')
actual_sha=$(sha256sum target/x86_64-unknown-linux-gnu/release/sysml-validate | awk '{print $1}')

if [ "$expected_sha" = "$actual_sha" ]; then
  echo "OK: byte-identical to published release."
else
  echo "MISMATCH: $actual_sha vs $expected_sha"
  exit 1
fi
```

## Diffoscope (when the digests differ)

If the SHA-256 check fails, `diffoscope` will tell you where the
binaries differ. Common culprits:

- An unpinned dependency that changed since the release tag. Fix:
  re-run with `--locked` and verify your local `Cargo.lock` matches the
  one in the tagged revision.
- A different Rust toolchain version. Fix: respect `rust-toolchain.toml`
  and don't pass `--toolchain` overrides.
- Build inside an unusual directory layout that defeats the
  `--remap-path-prefix` rules in [`.cargo/config.toml`](../.cargo/config.toml).
  Fix: build in a fresh clone, not on top of a long-lived workspace.
- Different glibc / linker on the host. For 100% bit-identical builds
  across hosts, build inside the Docker image documented at the
  bottom of this file.

```bash
diffoscope \
  --html-dir diffoscope.html \
  target/x86_64-unknown-linux-gnu/release/sysml-validate \
  /path/to/published/sysml-validate-x86_64-unknown-linux-gnu
```

## Cross-host reproducibility (Linux)

For the highest assurance — bit-identical across two different
machines — build inside a pinned container. The release workflow uses
`ubuntu-latest`; for independent verification, pin to a specific Ubuntu
runner image:

```bash
docker run --rm \
  -v "$PWD":/work -w /work \
  -e SOURCE_DATE_EPOCH \
  ghcr.io/rust-lang/rust:1.85.0-slim-bookworm \
  cargo build --release --locked --target x86_64-unknown-linux-gnu
```

A future revision of this document will document the exact image digest
once the first release ships and the digest is published.

## What is NOT promised

- macOS and Windows binaries are reproducible *given the same toolchain
  and SDK version*, but the SDK is host-specific and is not currently
  pinned by this repo. For audit-grade reproducibility on those
  platforms, also pin the host Xcode / Visual Studio Build Tools
  version. Track this in the corresponding GitHub Action runner image
  release notes.

- Reproducibility of any artifact NOT listed in the GitHub Release
  (e.g., a binary you compiled yourself with different flags) is not in
  scope.

- SBOM files (`*.cdx.json`, `*.spdx.json`) include generation
  timestamps and tool versions in their metadata. They are NOT
  bit-reproducible. The SBOM **contents** describe the same component
  set across runs; the JSON serialization itself may differ.

## CI verification

The release workflow runs `diffoscope` between two independent fresh
builds of the same tag and fails the release if they differ. See
[`.github/workflows/release.yml`](../.github/workflows/release.yml).

(NOTE: this `diffoscope` step is not yet wired in. Add a second build
job and a final compare step before treating the reproducibility claim
as load-bearing.)

## Related

- [`docs/SECURITY.md`](SECURITY.md) — signature and SBOM verification.
- [`docs/PRD-government-readiness.md`](PRD-government-readiness.md) —
  US-109 acceptance criteria.
