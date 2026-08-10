# Changelog — wolf.nvim

User-visible changes only (ls07 §4.6). Internal churn belongs in the campaign
closeout, not in the file a user reads before upgrading.

This file travels into the generated `wolf.nvim` mirror, which is why it says
"plugin" and not "the `clients/nvim/` subtree".

## 0.0.1 — UNRELEASED

**Never published.** `tenseleyFlow/wolf.nvim` does not exist yet; the mirror
split that would create its first commit is computed and verified locally by
`cargo xtask nvim-split` and pushed by nobody. Until then the plugin installs
from a `wolf-lsp` checkout (`README.md` §Installing).

- Wolf support for Neovim ≥ 0.11: filetype detection for `.lu`, an `lsp/wolf.lua`
  configuration for the built-in client, and `ftplugin` comment/indent settings.
- Hand-written `syntax/wolf.vim` highlighting, checked against the pinned
  grammar's keyword list.
- `:checkhealth wolf` reports the binary that won resolution, its version, the
  pin the plugin was verified against, and — new in this version — which side of
  the declared `wolf` range that binary is on. It warns; it never refuses to
  attach, and it never calls a version "unsupported".
- `:h wolf.nvim` works on a fresh install: `doc/tags` is committed.
- `vim.g.wolf.serverPath` overrides binary resolution.

Verified against `wolf 0.0.1 (pre-alpha)` at wolf-lang `70bdd35` — see
[`docs/COMPAT.md`](../../docs/COMPAT.md).
