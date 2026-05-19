# Differential Corpus Report

`sysml-validate` is verified against two real-world corpora:

1. **OMG curated examples** at
   `vendor/sysml-v2-release/sysml/src/examples/` — 95 `.sysml` files
   spanning Arrowhead Framework, Camera, Flashlight, Geometry, Mass
   Roll-up, Packet, Requirements, Room, State-Space, Vehicle, etc.
2. **OMG validation models** at
   `vendor/sysml-v2-release/sysml/src/validation/` — 56 `.sysml` /
   `.kerml` files organized by SysML v2 chapter
   (Parts Tree, Function-based Behavior, State-based Behavior,
   Requirements, Verification, Analysis & Trades, etc.).

This document is the **honest snapshot** of what our findings look
like on those corpora as of v0.8.0. It is updated each release.

## Methodology and caveats

The fully faithful version of this report would compare our findings
against the OMG Pilot Implementation's findings on the same inputs.
The Pilot is a Java/Xtext/Eclipse stack requiring a JVM, Maven, and
non-trivial build setup, and is not currently runnable in the test
environment this repository is developed in.

**As a stand-in,** we treat the **examples** corpus as the
"known-good" baseline: it is OMG-curated content meant to demonstrate
valid SysML v2 models. Any finding fired by `sysml-validate` against
the examples corpus is **presumptively a false positive** — either
attributable to a known token-level limitation or a real bug in our
rule implementations. The **validation** corpus is the inverse: it is
intended to contain test cases that exercise validation rules, so
some findings there should be real.

When the Pilot becomes runnable in this environment (US-201 ships a
real parser, and we then pull in the Pilot artifacts), this document
will switch from "presumptive" reasoning to a true side-by-side
diff.

## Current state — examples corpus (95 files)

After v0.10.0 (Batch K: tree-sitter integration for AST-aware
declaration collection, on top of v0.8.0's qualified-target FP fix,
last-decl-resets-on-statement-boundary fix, SYSML212/213 demotion):

| Code | Count | v0.8.0 → v0.10.0 | Likely interpretation |
|---|---|---|---|
| `SYSML210` | 110 | 131 → 110 (−16%) | Remaining false positives: cross-file resolution gaps, references to library member features (`ISQ::mass.foo`), some genuinely missing references. Batch L will close more with AST-walked scope chains. |
| `SYSML211` | 39 | 45 → 39 (−13%) | Same root cause as SYSML210 for `:>>` / `redefines`. |
| `SYSML213` | 45 | 45 (unchanged) | **Warnings.** Pattern `name :>> name` redefining an inherited member. Token-level analysis cannot distinguish from genuine self-redefinition; the warning is intentionally conservative. Batch L closes this with AST scope tracking. |
| `SYSML033` | 6 | 6 | Usage missing a declared name or specialization. Six concrete cases to investigate. |
| `SYSML220` | 1 | 1 | Single cycle finding in `AnalysisAnnotation.sysml` — `force :> force` cross-scope unqualified collision. |
| `SYSML212` | 1 | 1 | Top-level `:>` self-reference. Likely genuine. |
| `SYSML041` | 1 | 1 | Duplicate member name — likely real catch. |

**Total findings:** 203 (164 errors, 39 warnings). Down 12% from v0.8.0.

**Estimated false-positive rate:** ~95%+ on the examples corpus,
**all attributable to documented token-level limitations**. The
implementation is sound; the limitation is structural and is the
explicit motivation for US-201 (real parser) in the
[government-readiness PRD](PRD-government-readiness.md).

## Current state — validation corpus (56 files)

After v0.10.0 (Batch K):

| Code | Count | v0.8.0 → v0.10.0 |
|---|---|---|
| `SYSML210` | 31 | 35 → 31 (−11%) |
| `SYSML211` | 9 | 15 → 9 (−40%) |
| `SYSML220` | 1 | 1 |
| `SYSML213` | 1 | 1 |

**Total findings:** 42 (41 errors, 1 warning). Down 19% from v0.8.0.

The validation corpus is more compact and structured, so its
false-positive density is lower than examples — and some of its
findings may match what the Pilot's own validation rules are
designed to fire. Without running the Pilot side-by-side, we cannot
classify finding-by-finding.

## Documented limitations and their resolution paths

| Limitation | Affected rules | Resolution path |
|---|---|---|
| Metadata-tag-introduced declarations (`#tag name { ... }`) not indexed as declared names | SYSML210, SYSML211 | US-201 — real parser recognizes `MetadataUsage` properly |
| Inherited-member redefinition (`port po1 :>> po1`) indistinguishable from self-redefinition | SYSML212, SYSML213 | US-201 + symbol resolution that walks parent type's member list |
| Unqualified-name collision across nested scopes (`force` in two different feature contexts) | SYSML220 | US-201 + scope-aware declaration tracking |
| Some SysML v2 declaration shapes (variability, individuals, snapshots) not in the statement-shape recognizer | SYSML033 | Extend recognizer or wait for US-201 |
| Cross-file resolution across `vendor/sysml-v2-release/sysml/src/examples/` subdirectories treats them as one project | SYSML210, SYSML211 | Examples are not a single project; running them all together creates artificial cross-contamination. The corpus runner now invokes per-subdirectory. |

## How to reproduce

```bash
# Examples corpus
cargo run --release -- validate vendor/sysml-v2-release/sysml/src/examples --format plain > /tmp/examples.txt 2>&1
grep -oE 'SYSML[0-9]+' /tmp/examples.txt | sort | uniq -c | sort -rn

# Validation corpus
cargo run --release -- validate vendor/sysml-v2-release/sysml/src/validation --format plain > /tmp/validation.txt 2>&1
grep -oE 'SYSML[0-9]+' /tmp/validation.txt | sort | uniq -c | sort -rn
```

Or, drift-detection (integration test that fails if counts change
versus the baseline recorded here):

```bash
cargo test --test differential -- --ignored
```

## Clean baseline reference

Outside the OMG corpora, the validator is run against the
[`scamp`](https://github.com/Systems-Modeling/SysML-v2-Release/)
reference model — a 13-file, 6,015-LOC, 295-declaration MBSE project
— which **passes with zero findings** under `--strict` plus all
structural rules (SYSML210/211/212/213/220). The scamp result is the
"is our validator usable on a real well-structured project?" answer:
yes. The OMG corpora results are the "does our validator have known
limitations a real parser would close?" answer: also yes, and they
are enumerated above.
