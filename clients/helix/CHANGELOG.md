# Changelog — the Helix config fragment

User-visible changes only (ls07 §4.6).

The "artifact" here is `languages.toml`, a fragment a user copies into
`~/.config/helix/languages.toml`. It has no install channel and no version a
package manager can see, so this file's version number exists for exactly one
purpose: to let `docs/COMPAT.md` say which `wolf` the fragment was checked
against on which date.

## 0.0.1 — UNRELEASED

- `[[language]]` blocks for `wolf` (`.lu`) and `wolfi` (`.wolfi`), plus a
  `[language-server.wolf]` running `wolf lsp`.
- The `wolfi` block deliberately carries **no** `language-servers` key: `wolfi`
  v0 is a binary format and `wolf lsp` discovers modules by `.lu` alone (D32).
  Attaching the server there would produce a buffer that looks supported and is
  not.
- `[[grammar]]` ships **commented out**. `wolffe-lang/tree-sitter-wolf` is a
  seed commit with no `grammar.js`, so a `.lu` buffer in Helix has **no syntax
  highlighting**. Uncommenting it only makes Helix noisy at startup; it does not
  produce highlighting.
- No runtime version check, and there cannot be one: a TOML fragment cannot run
  code. [`docs/COMPAT.md`](../../docs/COMPAT.md) is the whole compatibility
  statement for this client.

Verified against `wolf 0.0.1 (pre-alpha)` at wolf-lang `70bdd35`, helix 25.07.1.
