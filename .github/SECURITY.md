# Security Policy

The full vulnerability-handling and release-verification policy lives
in [`docs/SECURITY.md`](../docs/SECURITY.md). This file is the
GitHub-surface short form.

## Reporting a vulnerability

Please use **GitHub's private vulnerability reporting** to disclose
security issues privately:

- <https://github.com/coldboot-studio/sysml_cli/security/advisories/new>

Do **not** open a public GitHub issue, post in Discussions, or send
the report to a public mailing list for security problems. A private
advisory keeps the disclosure under embargo while a fix is prepared.

## Encrypted reports

If you would prefer to send a PGP-encrypted report, the maintainer's
public key is published at `keys.openpgp.org`:

- **Fingerprint:** `A28BF7 2181 330C EFE8 F24B  11B2 FFD0 F520 4264 46`
- **Fetch:** `gpg --keyserver keys.openpgp.org --recv-keys A28BF72181330CEFE8F24B11B2FFD0F520426446`

The same key signs the project's release artifacts; see
[`docs/SECURITY.md`](../docs/SECURITY.md) for the release-verification
recipe.

## Supported versions

Only the most recent minor release line of `sysml-validate` receives
security fixes. The full support matrix is in
[`docs/SECURITY.md`](../docs/SECURITY.md).
