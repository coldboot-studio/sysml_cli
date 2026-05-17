# sysml-cli

`sysml-validate` is a command-line validation tool for SysML v2 and KerML textual
models. It is aligned with the public release layout from
[Systems-Modeling/SysML-v2-Release](https://github.com/Systems-Modeling/SysML-v2-Release):

- textual grammar references live in `bnf/`, including `SysML-textual-bnf.kebnf`
  and `KerML-textual-bnf.kebnf`
- example projects live in `sysml/` and `kerml/`
- normative model libraries live in `sysml.library/`

The native validator is intentionally conservative: it catches deterministic
textual issues without pretending to replace the full reference implementation.
For full conformance checking, use `--backend official` with a local command
that invokes the SysML v2 pilot/release tooling.

## Install

```powershell
python -m pip install -e .
```

The Rust implementation can be built without external crates:

```powershell
cargo build --release
```

## Usage

Validate files or directories:

```powershell
sysml-validate validate .\model.sysml
sysml-validate validate .\sysml .\kerml --format json
```

Use the official backend through a command template. `{file}` is replaced with
each model path.

```powershell
sysml-validate validate .\model.sysml --backend official --official-command "sysml-validator {file}"
```

Show the release conformance references encoded in the tool:

```powershell
sysml-validate grammar-info
```

Rust equivalent:

```powershell
cargo run -- validate .\examples
cargo run -- grammar-info
cargo run -- corpus-info
```

## Public Test Models

The Rust CLI includes `corpus-info` to list online SysML v2 model corpora that are
useful for smoke tests:

- `Systems-Modeling/SysML-v2-Release`: official examples, training, validation,
  and library models.
- `GfSE/SysML-v2-Models`: community-curated textual SysML v2 models.
- `sensmetry/advent-of-sysml-v2`: lesson-oriented SysML v2 examples.
- `sensmetry/smart-home-hub-example`: a small complete architecture example.
- OMG machine-readable SysML files, including the Simple Vehicle Model.

## Native Checks

The built-in backend validates:

- supported file extensions: `.sysml` and `.kerml`
- balanced braces, parentheses, and brackets
- unterminated string/block/comment constructs
- invalid control characters
- declaration terminators for common SysML/KerML constructs
- basic package/import/alias/dependency/definition/usage statement shape
- duplicate package member names in the same lexical scope
- optional unresolved qualified-name references with `--strict`

Exit codes:

- `0`: no errors
- `1`: validation errors found
- `2`: CLI or backend configuration error

## Scope

SysML v2 has a formal abstract syntax, semantic constraints, and model library
resolution rules. The native backend is useful in CI as a fast preflight gate,
but full language conformance should be delegated to the official release/pilot
implementation backend.
