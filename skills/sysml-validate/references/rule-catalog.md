# Rule catalog reference (for agents)

Every diagnostic `sysml-validate` emits carries a stable code in the
`SYSML\d{3}` namespace. The catalog version is bumped when a rule's
*meaning* changes; consumers can gate baselines on the version. Current
catalog version: **0.5.0**.

This reference is for AI agents helping users interpret and fix
findings. Human-audience material is in `docs/TECH_MANUAL.md` §7.

---

## How to use this table

1. The user shows you a diagnostic line (or a SARIF result, or a
   `SYSMLxxx` code in conversation).
2. Look up the code below.
3. Apply the **fix-pattern**; if a "common false-positive" row applies,
   investigate that before changing the model.
4. If the fix requires multiple steps, propose the minimal edit first
   and surface the remaining steps as follow-ups.

---

## Lexical rules

### SYSML001 — Invalid control character in source text (error)

**What it means.** A byte outside the printable / whitespace range was
found in the source file.

**Fix.** Strip the offending byte. Common cause: a model exported from
a Windows editor with a stray `\x0c` form-feed or `\x00` NUL.

**Check.** Confirm the file is UTF-8. `file model.sysml` should say
"UTF-8 Unicode text."

### SYSML002 — Unterminated string literal (error)

**What it means.** A `"` was opened but no matching `"` appears before
EOF or another statement boundary.

**Fix.** Close the string. If the string was meant to span lines,
remember that SysML/KerML string literals don't permit raw newlines —
escape with `\n`.

### SYSML003 — Unterminated block comment (error)

**What it means.** A `/*` was opened but no `*/` appears before EOF.

**Fix.** Close with `*/`. Avoid nested block comments; the grammar
doesn't permit them.

### SYSML010 — Unsupported file extension (error)

**What it means.** The file was passed to `validate` but doesn't end
in `.sysml` or `.kerml`.

**Fix.** Rename, or drop the file from `include` patterns in
`sysml-validate.toml`.

### SYSML012 — Unable to read file as UTF-8 text (error)

**What it means.** The file isn't valid UTF-8.

**Fix.** Re-save as UTF-8 (without BOM is fine; with BOM also works).

---

## Delimiter rules

### SYSML020 — Unmatched closing delimiter (error)

**What it means.** A `)` / `}` / `]` appeared with no matching opener.

**Fix.** Remove the extra closer, or add the missing opener earlier.
The error message gives the position.

### SYSML021 — Unclosed delimiter (error)

**What it means.** An opener (`(` / `{` / `[`) was never closed before
EOF or end of statement.

**Fix.** Add the matching closer. Usually the cause is a missing `}`
at the end of a `package` or `part def` body.

---

## Statement-shape rules

### SYSML030 — Expected `package` after `library` (error)

**What it means.** The keyword `library` must be followed by `package`.

**Fix.** `library package <Name>`, not `library <Name>`.

### SYSML031 — Alias declaration must include `for` (error)

**What it means.** Alias syntax is `alias <New> for <Existing>;`.

**Fix.** Add `for` between the alias name and the target.

### SYSML032 — Dependency must include a supplier after `to` (error)

**What it means.** Dependency syntax is `dependency <Name> to <Supplier>;`.

**Fix.** Provide a target after `to`.

### SYSML033 — Usage missing a declared name or specialization (error)

**What it means.** A usage statement like `part`, `attribute`, `action`
must either name a member or specialize one.

**Fix.** Either give it a name (`part wheel;`) or a specialization
(`part :> Engine;`). Both is also valid (`part w :> Engine;`).

**Common false positive (gated on US-201 AST migration).** Some
exotic usage shapes still trip the token recognizer; see
`docs/differential-corpus-report.md`.

### SYSML034 — Definition missing a name (error)

**What it means.** Definitions must be named.

**Fix.** `part def Engine`, `attribute def mass`, `action def burn` —
never bare `part def`.

### SYSML035 — Missing `;` or `{` terminator (error)

**What it means.** A statement was started but doesn't terminate
cleanly.

**Fix.** Add `;` (no body) or `{ ... }` (with body).

---

## Reference / scope rules

### SYSML040 — Identifier reference not resolvable (warning, under `--strict`)

**What it means.** A name was referenced (in `:>`, `:>>`, `:`, `for`,
`to`, `from`) that the resolver couldn't find via any of:
declared-in-file, imports (membership / namespace / recursive), other
files in the validation run, or the embedded SysML v2 standard library
(when `--strict` is on).

**Fix order.**
1. Check spelling.
2. If the name is in another file, add the file to the validation run
   (pass the directory, not the single file).
3. If the name is in a package, `import <Package>::*;` or qualify the
   reference (`<Package>::<Name>`).
4. If the name is in the standard library, ensure `--strict` is on (the
   library is only consulted under `--strict` — otherwise SYSML040 is
   suppressed for unresolved references).

### SYSML041 — Duplicate member name in lexical scope (error)

**What it means.** Two members share a name in the same lexical scope.

**Fix.** Rename one. If the duplicate is intentional (e.g.,
inherited-member redefinition), use `:>>` to mark it.

---

## Structural rules (SYSML100, SYSML2xx)

### SYSML100 — Parser could not understand region (warning, under `--strict` only)

**What it means.** The tree-sitter SysML grammar emitted an ERROR /
MISSING node for the flagged region.

**Important.** This rule is **gated behind `--strict`** because the
`tree-sitter-sysml` 0.1 grammar has incomplete coverage. A SYSML100
finding does not necessarily mean the source is invalid — it may be
a grammar-coverage gap. Inspect the snippet; if the syntax is legal
SysML/KerML, file an issue against the grammar.

### SYSML210 — `:>` / `specializes` target does not resolve (error)

**What it means.** The right-hand side of a specialization arrow
doesn't resolve.

**Fix.** Same fix-order as SYSML040, but this is an *error* not a
warning — the build fails until resolved.

### SYSML211 — `:>>` / `redefines` target does not resolve (error)

**What it means.** Same as SYSML210 but for redefinition.

**Fix.** Same as SYSML210.

### SYSML212 — Feature specializes itself (error)

**What it means.** `feature x :> x` — the feature is its own
generalization. This is invalid.

**Fix.** Either remove the `:>` (no specialization intended) or point
at a different feature. **Watch for the inherited-member pattern**:
inside an `*_usage` whose `usage_declaration` has a `typing_part`,
`feature x :> x` may be the legitimate "redefine the inherited
member" pattern. The AST-aware validator suppresses SYSML212 there,
but token-level cases can still slip through.

### SYSML213 — Feature redefines itself (error)

**What it means.** `feature x :>> x`. Same as SYSML212 but for
redefinition.

**Fix.** Same as SYSML212.

### SYSML220 — Specialization graph contains a cycle (error, project-wide)

**What it means.** Following `:>` edges across the project produces a
cycle: `A :> B :> C :> A`. Cycles break classification reasoning and
are rejected by the OMG Pilot.

**Fix.** Break the cycle. The diagnostic message names the full path
— pick one edge to remove or redirect.

---

## Suppression-directive rules

### SYSML050 — Suppression directive did not match any diagnostic (warning)

**What it means.** A `// sysml-validate: disable=...` directive
appeared on a line / context where no diagnostic was actually emitted
for the listed codes.

**Fix.** Remove the directive. It's dead.

### SYSML060 — Suppression directive has invalid syntax (warning)

**What it means.** The directive parse failed.

**Fix.** Use one of the four canonical forms:
- `// sysml-validate: disable=SYSML041`
- `// sysml-validate: disable=SYSML041,SYSML040`
- `// sysml-validate: disable-next-line=SYSML041`
- `// sysml-validate: disable=all`

See [suppression-syntax.md](suppression-syntax.md).

---

## Configuration / setup rules

### SYSML800 — Configuration file is invalid (error)

**What it means.** `sysml-validate.toml` failed to parse, or
contained an unknown key (the parser uses `deny_unknown_fields`).

**Fix.** The error message names the offending key. Check spelling
against the configuration reference in TECH_MANUAL.md §5.

---

## Official backend rules

### SYSML900 — `--official-command` parse/setup error (error)

**What it means.** The argv template passed to `--official-command`
couldn't be tokenized, or `{file}` was missing.

**Fix.** Ensure `{file}` appears in the template. Quote multi-word
tokens with `"..."`.

### SYSML901 — Official validator could not be executed (error)

**What it means.** The first argv token doesn't name an executable
the OS can find / launch.

**Fix.** Confirm the binary exists on `PATH` (or pass an absolute
path).

### SYSML902 — Official validator returned non-zero exit status (error)

**What it means.** The delegated validator ran but reported failure.

**Fix.** Inspect the child's stderr (it's drained and shown in the
diagnostic). This is usually a real issue in the model, not in
`sysml-validate`.

### SYSML903 — Official validator returned informational output (info)

**What it means.** The child wrote to stderr but exited cleanly. The
output is surfaced as an info diagnostic.

**Fix.** None required; this is non-failing.

### SYSML904 — Official validator exceeded `--timeout` (error)

**What it means.** The child was killed because it took longer than
`--timeout` seconds.

**Fix.** Increase `--timeout`, or investigate why the child hung
(usually: waiting on stdin, waiting on network).
