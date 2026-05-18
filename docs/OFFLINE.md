# Offline and Air-Gap Deployment

`sysml-validate` is designed for use inside DoD IL4/IL5 enclaves,
classified networks, and other air-gapped environments. This document
states the offline contract.

## The contract

**The `validate` subcommand makes zero network calls.**

This is not an option — it's a property of the implementation. The
validator reads source files from disk, optionally reads a
configuration file from disk, optionally reads a baseline SARIF from
disk, and emits diagnostics. Nothing else is touched at runtime.

## Subcommand-by-subcommand surface

| Subcommand        | Network calls | Files read | Files written |
|-------------------|---------------|------------|---------------|
| `validate`        | none          | input models, optional `sysml-validate.toml`, optional baseline SARIF | none (output to stdout); optional baseline write with `--update-baseline` |
| `grammar-info`    | none          | none       | none          |
| `corpus-info`     | none          | none       | none          |

`--backend official` invokes a user-supplied child process. That child
may do anything — we cannot constrain it. If you deploy in an
air-gapped environment with the official backend, verify the child
binary's own offline contract.

## No telemetry, no auto-update

- The binary does NOT phone home.
- The binary does NOT check for updates on startup or any other time.
- The binary does NOT emit usage analytics, crash reports, or any other
  telemetry signal to any endpoint, ever.

If you find behavior contradicting these statements, file a security
report per [`SECURITY.md`](SECURITY.md).

## Environment variables

Honored:

| Variable             | Effect |
|----------------------|--------|
| `NO_COLOR`           | Suppresses ANSI color in text output. |
| `SYSML_VALIDATE_*`   | Reserved for future use; none consumed today. |

Not honored:

- No proxy variables (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`) — the
  validator never opens an HTTP socket.

## Crate-fetch and first-build network use

Building from source requires network access *the first time* to
populate the Cargo registry and download crate sources. Once built, the
binary is fully offline.

For air-gapped build environments, vendor the dependency tree:

```bash
cargo vendor --locked vendor/
```

The resulting `vendor/` tree plus the source tree is sufficient for a
fully offline build. Configure cargo to use the vendored sources by
adding to `.cargo/config.toml`:

```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
```

We do not commit a vendored tree to this repository (it would 30× the
clone size). A sample CI job that produces an air-gap-ready tarball
lives in `.github/workflows/release.yml` as a future enhancement.

## Filesystem footprint

A single statically-linked binary plus, optionally, a
`sysml-validate.toml` configuration file. No state directory. No
caches. No logs other than what the operator captures from stdout.

## Verifying offline behavior empirically

```bash
# Linux: run under strace and confirm no socket calls.
strace -e trace=network -f -o syscalls.log \
  sysml-validate validate ./model.sysml --ci > /dev/null
grep -c '^' syscalls.log    # expected: 0
```

## Related

- [`SECURITY.md`](SECURITY.md) — release verification.
- [`THREAT_MODEL.md`](THREAT_MODEL.md) — trust boundaries.
