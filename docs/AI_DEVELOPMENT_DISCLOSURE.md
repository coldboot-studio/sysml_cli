# AI-Assisted Development — Disclosure

`sysml-validate` is partly authored with AI-assisted coding tools.
This disclosure exists because the project targets government and
other regulated adopters whose acquisition reviewers reasonably ask
how modern coding assistance is governed in software they rely on.

The position taken here is that the audit-relevant question is *how
code is verified*, not *what authoring tool generated it*. The
controls below are what an adopter should evaluate.

## What this means in practice

- **Human review is non-negotiable.** Every commit that lands on the
  default branch is reviewed by the named human author before merge.
  AI-assisted edits do not bypass review.
- **The release pipeline is the gate.** Every release artifact passes
  through, and fails closed on, the following CI checks:
  - `cargo audit` against the RustSec advisory database (NIST SSDF
    RV.1)
  - `cargo clippy --all-targets -- -D warnings` (clippy-style lints
    elevated to errors)
  - `cargo fmt --check` (uniform formatting)
  - `cargo test --locked` on Linux, macOS, and Windows runners
  - The project's own differential corpus harness against the
    OMG-curated example and validation corpora
- **Every shipped binary carries verifiable provenance.** Per release:
  a SHA-256 checksum, a Sigstore (cosign) keyless signature with
  Rekor inclusion proof, a detached GPG signature against the
  project's long-lived ed25519 key, CycloneDX 1.6 and SPDX 3.0
  SBOMs, and SLSA v1.0 Build Level 3 in-toto provenance attestation.
- **Reproducibility is enforced, not asserted.** The release workflow
  rebuilds the Linux x86_64 binary independently on a clean runner
  and uses `diffoscope --exit-code` to fail closed on any byte-level
  difference. See [`REPRODUCING.md`](REPRODUCING.md).
- **Multiple AI coding tools have been used** across the project's
  development. The project is intentionally not naming specific
  vendors in this document, but is also not concealing which tools
  were used. For specifics about which assistants contributed to
  which parts of the codebase, contact the maintainer per
  [`SECURITY.md`](SECURITY.md). The honest answer will be provided.

## Policy alignment

The posture above is consistent with:

- **NIST AI Risk Management Framework 1.0** — governs AI integration
  in operational systems. The project's use of AI is within scope of
  the framework's "GOVERN" and "MEASURE" functions; the verification
  pipeline implements both.
- **NIST SP 800-218A — Secure Software Development Practices for
  Generative AI and Dual-Use Foundation Models** — the SSDF-ML
  augmentation. SP 800-218A explicitly anticipates AI-assisted
  software development and frames trust in terms of verification
  controls rather than authorship provenance. The project's
  controls map to SSDF-ML practices PW.1 (design with security
  requirements), PW.4 (reuse with provenance), PW.7 (code review),
  PW.8 (test generated code), and RV.1 (identify vulnerabilities).
- **NIST SP 800-218 (SSDF, parent framework)** — full mapping in
  [`compliance/ssdf-mapping.md`](compliance/ssdf-mapping.md).
- **NIST SP 800-53 Rev 5 SA-11 (Developer Testing and Evaluation)** —
  satisfied by the testing + SAST controls listed above. Full
  mapping in [`compliance/nist-800-53-mapping.md`](compliance/nist-800-53-mapping.md).

## What this disclosure does *not* claim

- This disclosure does **not** claim that the project's authoring
  process is free of AI assistance. It is not.
- This disclosure does **not** claim that AI assistance has been
  limited to any particular subsystem or commit range. It has been
  used across the codebase.
- This disclosure does **not** assert that AI-generated code is
  inherently safer or less safe than hand-written code. The project
  asserts only that the *verification controls applied uniformly to
  all code* are what an adopter should evaluate.

## Open questions and updates

If an adopter has acquisition-review questions about AI-assisted
development that are not answered here, file a private discussion
via the repository's Security tab (private vulnerability reporting
will accept non-vulnerability disclosure questions) or contact the
maintainer per [`SECURITY.md`](SECURITY.md). This document will be
updated as government guidance on AI-assisted development matures.
