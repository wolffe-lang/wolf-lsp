---@brief
---
--- https://github.com/wolffe-lang/wolf-lang
---
--- `wolf lsp` is the wolf compiler's language-server subcommand. There is no
--- separate server to install: the binary that answers `initialize` is the
--- same one `wolf build` runs, so a diagnostic in the editor and a diagnostic
--- on the command line come from one code path by construction.
---
--- Install the wolf toolchain and put `wolf` on `PATH`. Nothing else is
--- required — no server download, no `mason` entry, no per-project settings.
---
--- To override the binary (a checkout's `target/release/wolf`, say), set `cmd`
--- rather than adding a settings key:
---
--- ```lua
--- vim.lsp.config('wolf', { cmd = { '/path/to/wolf', 'lsp' } })
--- ```
---
--- The `wolf.nvim` plugin does exactly that for you from `vim.g.wolf`
--- (`{ serverPath = … }`), and adds filetype detection, a regex syntax
--- fallback and `:checkhealth wolf`. This file works without it.

-- THIS FILE IS THE UPSTREAMABLE ENTRY.
--
-- Since nvim-lspconfig 2.0 an lspconfig server entry *is* an `lsp/<name>.lua`
-- returning a `vim.lsp.Config` table, with the `---@brief` block above as its
-- generated documentation. So there is no second copy of this table in a
-- second shape to keep in sync: dropping this file into
-- `neovim/nvim-lspconfig`'s `lsp/` directory is the whole port. Opening that
-- PR is ls07's, and it waits on wolf-lang publishing an installable release —
-- lspconfig reasonably declines entries for servers nobody can install.
--
-- Two fields a reader may expect and will not find:
--
--   * `single_file_support` — not a field in Neovim's native config, and
--     dropped by lspconfig 2.0 along with the rest of the framework layer.
--     Native single-file support is the DEFAULT: when no `root_markers` match,
--     `root_dir` is nil, and `vim.lsp.start` starts the client anyway unless
--     `workspace_required = true` (`runtime/lua/vim/lsp.lua`). So a scratch
--     `.lu` outside any package still diagnoses, and the way to say so is to
--     write nothing. A field set to its own default is a claim that stops
--     being true silently.
--   * `settings` — wolf's server reads none. It never sends
--     `workspace/configuration` and answers no settings key, so a `settings`
--     block here would be inert decoration. Behaviour belongs to the compiler
--     (D34); a client-side settings zoo is how editor layers acquire opinions
--     the language never agreed to.

return {
  -- The compiler is the language server (D34). `cmd` is resolved through
  -- `PATH` on purpose: the server a buffer talks to should be the same one the
  -- shell's `wolf build` finds, and an absolute path baked in here is how the
  -- two silently diverge.
  cmd = { 'wolf', 'lsp' },

  -- `wolf` only. `.wolfi` interface files get their own filetype (they are
  -- generated artifacts) and the server does not serve them at this pin.
  filetypes = { 'wolf' },

  -- List order is priority (`:h lsp-root_markers`): the nearest ancestor with
  -- a `wolf.pkg` wins; failing that, the nearest ancestor with a `.git`.
  -- Nesting them (`{ { 'wolf.pkg', '.git' } }`) would make them equal and let
  -- a `.git` in a parent of the package beat the package manifest.
  root_markers = { 'wolf.pkg', '.git' },
}
