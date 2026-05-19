# Third-Party Notices

`sysml-validate` itself is distributed under the MIT License (see
[`LICENSE`](LICENSE)). It embeds and depends on the following third-
party software at distribution time.

## Vendored: OMG SysML v2 Standard Library

- **Path in this repo**: `vendor/sysml-v2-release/sysml.library/`
- **Source repository**: <https://github.com/Systems-Modeling/SysML-v2-Release>
- **Pinned release tag**: `2026-04`
- **Pinned commit**: `9baca5908ca28b53da085de69336fde48420ea8f`
- **License**: **Eclipse Public License v2.0 (EPL-2.0)**
- **Upstream license file**: `vendor/sysml-v2-release/LICENSE`

The SysML v2 standard library is a normative artifact of the OMG SysML
2.0 specification (`formal/26-03-02`). It is redistributed here under
EPL-2.0. EPL-2.0 permits redistribution in source and binary form,
including embedding in a larger work, provided that the EPL-2.0 notice
is preserved alongside the redistributed material. The redistribution
in this repository does **not** alter the SysML v2 standard library;
all files in `vendor/sysml-v2-release/sysml.library/` are byte-identical
to the upstream release tag.

The remainder of `sysml-validate` is not covered by EPL-2.0; only the
embedded standard library content is.

When bumping the pinned release tag, update:

1. The submodule pin in `.gitmodules` and the working tree.
2. The "Pinned release tag" and "Pinned commit" entries above.
3. The pinned tag reference in [`docs/REPRODUCING.md`](docs/REPRODUCING.md).
4. The library version reported by `sysml-validate library-info`.
5. The CHANGELOG with a brief description of upstream changes.

## Rust crate dependencies

The Cargo dependency graph is enumerated in the CycloneDX 1.6 and SPDX
3.0 SBOMs attached to every release (see
[`docs/SECURITY.md`](docs/SECURITY.md)). Top-level direct dependencies
and their licenses are listed in `Cargo.toml`. None of those crates are
embedded as vendored source; they are fetched from `crates.io` at build
time per the locked `Cargo.lock`.
