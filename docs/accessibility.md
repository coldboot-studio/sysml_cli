# Accessibility and Section 508 Conformance

This document describes the accessibility posture of `sysml-validate`
and contains a draft VPAT 2.5 (Revised 508) self-assessment. The VPAT
draft is suitable for federal procurement initial review; a final
production VPAT should be reviewed by an accessibility consultant
before being signed and dated for an RFP submission.

## Applicable standards

- **Section 508 of the Rehabilitation Act, as amended.** Federal ICT
  procurement obligation under 29 U.S.C. § 794d.
- **Revised 508 Standards (36 CFR Part 1194)**, which incorporate
  **WCAG 2.0 Level A and AA** for web and software ICT.
- **EN 301 549 v3.2.1** (international equivalent, harmonized with WCAG
  2.1 AA). Not required for US federal procurement but reduces friction
  with allied-government procurement.

## What `sysml-validate` is, for accessibility purposes

A non-interactive command-line tool. Per the Revised 508 Standards,
this is **"non-web software"** (Chapter 5). It has no graphical user
interface, no keyboard navigation beyond the host terminal, no visible
focus indicators to manage, and no time-based media.

The relevant obligations reduce to:

1. **No information conveyed by color alone.** Severity is always
   carried by the literal text `ERROR` / `WARNING` / `INFO`, never
   only by a color.
2. **Output is parseable by assistive technology (screen readers).**
3. **Output respects user-controlled environment variables** that
   reduce visual noise.
4. **Documentation is in an accessible format.** All project docs are
   Markdown; rendered as HTML by GitHub, screen-reader-friendly.

## Implementation

| Obligation | How `sysml-validate` satisfies it |
|---|---|
| No color-only signaling | The tool emits zero ANSI escape sequences as of v0.4.0. Severity is always the literal text `ERROR`, `WARNING`, or `INFO`. Verified by grep over `src/`. |
| Screen-reader-parseable output | `--format plain` produces one diagnostic per line in the well-established GCC compiler format: `<path>:<line>:<column>: <severity>: <code>: <message>`. This format has decades of screen-reader and IDE support. |
| `NO_COLOR` environment variable | The community standard `NO_COLOR` (https://no-color.org/) is trivially honored — no color is emitted today regardless. The contract is documented for the day TTY-aware coloring is added. |
| Deterministic, parseable output for assistive tools | All five output formats (`text`, `plain`, `json`, `sarif`, `junit`) are deterministic and free of escape sequences. |
| Documentation | Markdown source with semantic headings, alt text where images appear (none currently), plain English. |

## Draft VPAT 2.5 (Revised 508)

The full VPAT template is published at
https://www.itic.org/policy/accessibility/vpat. The relevant chapters
for a pure CLI:

### Chapter 3 — Functional Performance Criteria (302)

| § | Criterion | Conformance | Notes |
|---|---|---|---|
| 302.1 | Without Vision | Supports | All output is text; `--format plain` is the screen-reader path. |
| 302.2 | With Limited Vision | Supports | No color-only signaling; severity is literal text. |
| 302.3 | Without Perception of Color | Supports | No color emitted as of v0.4.0. |
| 302.4 | Without Hearing | Not Applicable | No audio output. |
| 302.5 | With Limited Hearing | Not Applicable | No audio output. |
| 302.6 | Without Speech | Not Applicable | No voice input or output. |
| 302.7 | With Limited Manipulation | Supports | CLI; manipulation accommodations are the responsibility of the host terminal emulator. |
| 302.8 | With Limited Reach and Strength | Supports | Same as 302.7. |
| 302.9 | With Limited Language, Cognitive, and Learning Abilities | Supports | Diagnostics include both a stable code (`SYSML041`) and a plain-English message. The README maps every code to a description. |

### Chapter 5 — Software (502, 503, 504)

| § | Criterion | Conformance | Notes |
|---|---|---|---|
| 502 | Interoperability with Assistive Technology | Supports | The tool runs in any standard terminal. It emits no escape sequences that would confuse a screen reader. Output to a pipe (the typical AT capture point) is identical to terminal output. |
| 502.2.1 | User Control of Accessibility Features | Supports | The host terminal owns accessibility; the tool does not override its settings. |
| 502.2.2 | No Disruption of Accessibility Features | Supports | Same. |
| 502.3.1 | Object Information | Supports | Every diagnostic includes a stable rule ID and plain text. |
| 502.3.2 | Modification of Object Information | Not Applicable | Read-only diagnostic emitter. |
| 502.3.3 | Row, Column, and Headers | Not Applicable | No tabular UI. |
| 502.3.4 | Values | Supports | Severity and code are textual fields, not glyphs or color. |
| 502.3.5 through 502.3.14 | Modification, Label Relationships, Hierarchical Relationships, Text, List of Actions, Actions on Objects, Focus Cursor, Modification of Focus Cursor, Event Notification | Not Applicable | No UI. |
| 503 | Applications | Supports | The tool does not override platform accessibility features. |
| 503.2 | User Preferences | Supports | Honors `NO_COLOR`. Has not yet been audited against `--format plain` for additional preference variables; pending Phase 2 LSP work which may add more. |
| 503.3 | Alternative User Interfaces | Not Applicable | The tool has no GUI. |
| 503.4.1 | Caption Controls | Not Applicable | No video. |
| 503.4.2 | Audio Description Controls | Not Applicable | No video. |
| 504 | Authoring Tools | Not Applicable | `sysml-validate` does not author content. |

### Chapter 6 — Support Documentation and Services

| § | Criterion | Conformance | Notes |
|---|---|---|---|
| 601.1 | Scope | Supports | This document and the README are the support documentation. |
| 602.2 | Accessibility and Compatibility Features | Supports | This document. |
| 602.3 | Electronic Support Documentation | Supports | Markdown documents rendered by GitHub as HTML, navigable by keyboard, screen-reader-friendly. |
| 602.4 | Alternate Formats for Non-Electronic Support Documentation | Not Applicable | No non-electronic documentation. |
| 603 | Support Services | Partially Supports | Support is via GitHub issues. GitHub itself meets Section 508. There is no separate accessible-only support channel; the issue tracker, security advisories, and email all serve. |

### WCAG 2.0 A and AA (Chapter 5 cross-reference)

For a pure CLI, most WCAG 2.0 success criteria are inapplicable (they
target web content). The applicable ones:

| SC | Description | Status |
|---|---|---|
| 1.4.1 | Use of Color | Supports — no information by color alone. |
| 3.3.1 | Error Identification | Supports — every diagnostic states the error in text. |
| 3.3.3 | Error Suggestion | Partially Supports — most rules' messages explain the violation but do not propose a fix. Phase 2 quick-fix support in the LSP will improve this. |

## Known limitations

- The default `text` format includes a one-line metadata header. While
  this is screen-reader-parseable, users on screen readers may prefer
  `--format plain` which omits the header.
- Some Windows paths begin with `\\?\` (the canonical UNC prefix
  emitted by `fs::canonicalize`). This is unusual to read aloud. A
  future revision may strip the prefix from human-readable formats
  while preserving it in machine formats. Tracking this as a Phase 2
  follow-up.
- This document is a **draft** VPAT. A final production VPAT for an
  RFP response should be reviewed by an accessibility consultant.

## How to test for yourself

```bash
# Confirm no ANSI escape sequences are emitted.
sysml-validate validate ./model.sysml --format plain | cat -v | grep -c '\^\['
# Expected output: 0

# Confirm severity is literal text, not color or symbol.
sysml-validate validate ./model.sysml --format plain
# Expected: lines like
#   model.sysml:5:1: error: SYSML041: Duplicate member name 'Engine' ...

# Confirm NO_COLOR is honored (today: trivially, since no color is ever
# emitted; documented for future TTY-color additions).
NO_COLOR=1 sysml-validate validate ./model.sysml
```

## Related

- [`docs/SECURITY.md`](SECURITY.md) — release-verification commands;
  the verification recipes are also accessible.
- [`README.md`](../README.md) — diagnostic code catalog; every rule has
  a textual description that explains the violation.
