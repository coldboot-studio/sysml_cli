# Threat Model

This document enumerates the trust boundaries `sysml-validate` defends
and the boundaries it does not. It is intended for security reviewers
performing ATO, CMMC L2, or SCRM evaluations.

## Assets

1. **The validator binary itself** — must be authentic and unmodified
   between build and execution.
2. **The diagnostic output** — must accurately represent the model's
   conformance state; consumers rely on it to gate releases.
3. **The host filesystem** — the validator reads model files, an
   optional config file, and an optional baseline; it must not read or
   write outside those.
4. **The host network** — must not be touched at all by the `validate`
   subcommand (see [`OFFLINE.md`](OFFLINE.md)).

## Trust boundaries

### Boundary 1: build → release

**Threat.** An attacker tampers with the binary between build and
distribution.

**Mitigation.**
- Build runs on GitHub-hosted runners with documented provenance.
- Every release artifact is signed twice: Sigstore keyless (Rekor-
  logged) and a long-lived GPG key.
- SLSA v1.0 Build Level 3 in-toto attestation names the source commit
  and workflow file.
- CycloneDX 1.6 + SPDX 3.0 SBOMs enumerate every component.
- The build is reproducible (see [`REPRODUCING.md`](REPRODUCING.md));
  independent rebuilders can verify byte-identity without trusting
  GitHub.

**Residual risk.** A compromise of GitHub Actions' OIDC identity could
sign a malicious binary. Mitigation: Sigstore Rekor records every
signing event in a public transparency log; out-of-band monitoring can
flag unexpected entries.

### Boundary 2: user-controlled inputs → validator

**Threat.** A maliciously crafted model file, configuration file, or
baseline SARIF could exploit the parser or output emitter.

**Mitigation.**
- The parser is a hand-rolled lexer with no `unsafe` Rust.
- Configuration is parsed with `toml` (`deny_unknown_fields`).
- Baseline SARIF is parsed with `serde_json`.
- All file reads are bounded by available memory; no streaming-eval
  paths.

**Residual risk.** A pathological input could exhaust memory or CPU.
The `--timeout` flag (US-108) bounds the official backend; the native
backend is single-pass and bounded by input size, but is not hardened
against memory-blowup adversarial inputs. Treat untrusted input the
same way you would treat untrusted source code in any linter.

### Boundary 3: `--official-command` → child process

**Threat.** The `--official-command` template could be used to inject
shell metacharacters and execute arbitrary commands.

**Mitigation.**
- The template is tokenized with a shell-style parser (`shlex_split`)
  and invoked via positional argv. **No shell process is ever
  spawned.** Shell metacharacters in the template survive only as
  literal argv content.
- `{file}` substitution replaces a single token; it cannot split into
  multiple argv elements.
- See [`src/backend.rs`](../src/backend.rs) for the implementation and
  unit tests covering the injection-resistance properties.

**Residual risk.** The child process the user invokes is fully trusted
once spawned. We do not sandbox it; we do enforce a `--timeout`
deadline and a SIGKILL if it overruns.

### Boundary 4: filesystem reads/writes

**Threat.** A symlink or junction could redirect a read into an
unintended file, or a write into an unintended path.

**Mitigation.**
- Reads use `fs::read_to_string` which follows symlinks. We do not
  attempt to canonicalize away symlinks; users who care should not
  validate trees with adversarial symlinks.
- The only writes are: (a) optional `--update-baseline` to the path
  the user specified, (b) the `--official-command` child does whatever
  it does.

### Boundary 5: validator output → downstream consumer

**Threat.** A malicious diagnostic message could include
control characters or shell metacharacters that affect downstream
processing.

**Mitigation.**
- JSON and SARIF output uses `serde_json` which escapes all control
  characters per the JSON spec.
- Text output emits the raw diagnostic message; consumers piping into
  shell scripts must quote appropriately.
- XML (JUnit) output escapes `& < > " '`.

## Out of scope

The following are **not** defended by this tool. Consumers needing
defenses against these should add controls in the surrounding pipeline.

- **The integrity of `cargo` and the Rust toolchain.** We trust the
  Rust toolchain we depend on. SLSA L3 covers our build, not Rust's.
- **The integrity of the Sigstore root.** A compromise of Sigstore's
  certificate transparency log would invalidate the cosign trust path.
  Cross-check with GPG signatures or with SLSA provenance.
- **Side channels** (timing, EM, etc.). Not a meaningful risk for a
  static analysis tool.
- **Denial of service against the OFFICIAL backend.** Bring your own
  rate limiting; we provide `--timeout` per file but not concurrency
  limits.
- **The model authors.** This tool reports diagnostics; it does not
  attest to model truthfulness. Models that describe untruthful
  systems will still validate.

## Related

- [`SECURITY.md`](SECURITY.md) — verification recipes and disclosure
  process.
- [`OFFLINE.md`](OFFLINE.md) — network-touch surface area.
- [`PRD-government-readiness.md`](PRD-government-readiness.md) — full
  roadmap including the NIST 800-53 control-mapping appendix.
