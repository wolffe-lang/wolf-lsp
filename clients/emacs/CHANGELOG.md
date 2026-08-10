# Changelog — wolf-mode.el

User-visible changes only (ls07 §4.6).

The artifact is one elisp file, quoted verbatim in `README.md` so it can be
pasted rather than installed. There is no MELPA recipe at v1 (ls07 non-target)
and no `package.el` archive, so this version number exists to date the
compatibility row in `docs/COMPAT.md` and nothing else.

## 0.0.1 — UNRELEASED

- `wolf-mode`, a derived `prog-mode` for `.lu`: font-lock for keywords, types
  and doc comments, comment syntax, and an `eglot` server program entry running
  `wolf lsp`.
- `.wolfi` is deliberately **not** associated with the server (D32), for the
  same reason it is not in the Helix, Neovim, VS Code or Zed configurations.
- No tree-sitter mode: `tenseleyFlow/tree-sitter-wolf` has no grammar, so
  highlighting is font-lock keywords and nothing more.
- No runtime version check, by design. A snippet a user pastes into `init.el`
  that then compares versions against a range is a snippet nobody pastes;
  [`docs/COMPAT.md`](../../docs/COMPAT.md) carries the statement instead.

Verified against `wolf 0.0.1 (pre-alpha)` at wolf-lang `70bdd35`,
GNU Emacs 30.2 with eglot 1.17.30.
