# Suppression directive syntax (for agents)

This reference is the authoritative grammar for `sysml-validate`
suppression directives. **Use it verbatim** — do not invent variants.

## Grammar

A suppression directive is a single line comment with exact prefix
`// sysml-validate:`:

```
// sysml-validate: disable=<RULE>
// sysml-validate: disable=<RULE1>,<RULE2>,...
// sysml-validate: disable-next-line=<RULE>
// sysml-validate: disable-next-line=<RULE1>,<RULE2>,...
// sysml-validate: disable=all
```

Whitespace between `//` and `sysml-validate:` is **not** flexible
beyond a single space (the scanner is strict). Whitespace between
`disable=` and the code list is also not permitted — `disable= SYSML041`
is invalid.

`<RULE>` is a literal `SYSMLxxx` code from the rule catalog. Comma-
separated lists must not contain whitespace inside the list.

## Scope semantics

| Directive | Lines affected |
|---|---|
| `disable=<CODE>` (same-line form) | The line the directive sits on. |
| `disable-next-line=<CODE>` | The next *non-blank* line after the directive. |
| `disable=all` (same-line) | The directive's line, every rule. |
| `disable-next-line=all` | The next non-blank line, every rule. |

"Non-blank" means containing at least one non-whitespace character.
Blank lines (including blank lines that hold only a comment) are
skipped.

## Effect

- Suppressed diagnostics are **kept on the result list** but marked
  with their suppression reason.
- They are **excluded from text and JSON output by default**.
- They **always appear in SARIF** as `results[].suppressions[]`
  entries with `kind: "inSource"` and `status: "accepted"` (this is
  the SARIF audit record — required by the spec).
- They **never affect the exit code**, regardless of `--fail-on-warning`.

Pass `--show-suppressed` to surface them in text and JSON output too.

## Warnings the validator emits about directives

### SYSML050 — directive matched no diagnostic

If a directive is in source but the code it names didn't actually
fire on the line in question, `SYSML050` warns. **Action:** remove
the dead directive.

Example:

```sysml
// sysml-validate: disable=SYSML041
part def Engine;        // SYSML041 doesn't fire here — directive is dead
```

### SYSML060 — invalid directive syntax

If a directive looks like a suppression but doesn't parse, `SYSML060`
warns. **Common causes:**

- `# sysml-validate: ...` — wrong comment prefix; SysML/KerML uses `//`, not `#`. `#` introduces metadata tags.
- `// sysml-validate: disable SYSML041` — missing `=` between `disable` and the code.
- `// sysml-validate: disable=SYSML 041` — whitespace in the code.
- `// sysml-validate: disable= SYSML041` — whitespace after `=`.
- `// sysml_validate: disable=...` — underscore instead of hyphen.
- `// sysml-validate: ignore=...` — `ignore` isn't a directive keyword; only `disable` and `disable-next-line` are.

## Examples — every legal form

```sysml
package P {
  // same-line, single rule
  part def Engine; part def Engine;             // SYSML041 suppressed below
  // sysml-validate: disable=SYSML041

  // same-line, multiple rules
  // sysml-validate: disable=SYSML041,SYSML040
  part w :> NotFound; part w :> NotFound;

  // next-non-blank-line, single rule
  // sysml-validate: disable-next-line=SYSML040

  part wheel :> Missing;

  // next-non-blank-line, all rules
  // sysml-validate: disable-next-line=all

  alias E for Engine;

  // same-line, all rules
  alias F for G;    // sysml-validate: disable=all
}
```

## When to recommend suppression vs. configuration

| Situation | Mechanism |
|---|---|
| One specific occurrence is a known false positive | Inline directive (`disable=`) |
| A whole rule should be silenced project-wide | `sysml-validate.toml` `[rules] CODE = "off"` |
| A whole rule should be promoted to error project-wide | `sysml-validate.toml` `[rules] CODE = "error"` |
| Adopting the validator without fixing legacy findings | Baseline mode (`--baseline`), **not** mass-suppression directives |

Mass-suppression with inline directives is an anti-pattern. If you
find yourself recommending the user add a directive to every other
line, push them toward baseline mode instead.
