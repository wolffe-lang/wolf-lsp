-- Buffer-local defaults for wolf source.
--
-- Every value here is READ OFF the toolchain, not chosen: the indent width and
-- the line width are `wolf_fmt`'s constants, the comment forms are the
-- grammar's, and anything the toolchain does not fix is left alone. A filetype
-- plugin that invents defaults is a second style authority, which is the exact
-- thing a zero-option formatter exists to prevent (D34).
--
-- `:h ftplugin` runs this on every wolf buffer, so the guard and the undo list
-- are mandatory.

if vim.b.did_ftplugin then
  return
end
vim.b.did_ftplugin = true

local undo = {
  'setlocal comments< commentstring< formatoptions< formatprg<',
  'setlocal expandtab< shiftwidth< softtabstop< tabstop< textwidth< iskeyword<',
  'setlocal foldmethod< foldexpr<',
}
vim.b.undo_ftplugin = table.concat(undo, ' | ')

-- ------------------------------------------------------------- comments --

-- `[gram.lex.comment]`: `//` line comments, with `///` (item doc) and `//!`
-- (inner doc) as prefixes of the same form. **Wolf has no block comment** —
-- "nesting arguments lose to simplicity + lexer speed" — so `commentstring`
-- has no `/* %s */` alternative and nothing here should offer one.
vim.bo.commentstring = '// %s'
-- Ordered longest-first: `comments` is matched in order, so `:///` before
-- `://!` before `://` keeps a doc comment from being re-flowed as a plain one.
vim.bo.comments = ':///,://!,://'

-- ---------------------------------------------------------------- style --

-- `wolf_fmt::doc::INDENT` — 4, locked by s11, changes only at an edition
-- boundary (D36). Spaces: every byte of canonical output is a space, and a tab
-- in a wolf file is a formatting diff waiting to happen.
vim.bo.expandtab = true
vim.bo.shiftwidth = 4
vim.bo.softtabstop = 4
vim.bo.tabstop = 4

-- `wolf_fmt::doc::WIDTH` — 100. Set so `gq`, `colorcolumn` and the ruler agree
-- with the formatter…
vim.bo.textwidth = 100
-- …but with 't' removed, so it never HARD-WRAPS code as you type. `textwidth`
-- plus the default `formatoptions=tcqj` would break a 101-column expression in
-- half mid-insert, which the formatter would then have to un-break. 'c' keeps
-- comment auto-wrapping, 'q' allows `gq` on comments, 'j' joins them cleanly,
-- 'r'/'o' continue the comment leader.
vim.bo.formatoptions = 'croqlj'

-- `IDENT ::= ('_' XID_Continue+) | (XID_Start XID_Continue*)`. Neovim's
-- 'iskeyword' cannot express XID beyond latin-1, so this is the closest
-- expressible superset: word chars, digits, underscore, and 0xC0–0xFF. An
-- identifier using a CJK or Greek XID_Start still highlights (the syntax file
-- matches on `\k` plus `\w`), but `w`/`*` motions clip it. That is a Neovim
-- limit, not a wolf one, and it is written down rather than papered over.
vim.bo.iskeyword = '@,48-57,_,192-255'

-- -------------------------------------------------------- formatting --

-- Two paths, never three (the third would be a reimplementation of the style):
--
--   1. LSP `textDocument/formatting`, which `wolf lsp` serves. Neovim installs
--      it itself: `lsp._set_defaults` sets `formatexpr` to
--      `v:lua.vim.lsp.formatexpr()` on attach IF the option is still empty.
--      So this file deliberately does NOT set `formatexpr` — setting it is how
--      you lock the LSP path out of `gq` forever.
--   2. `wolf fmt -`, for a buffer with no server attached. `formatprg` is only
--      consulted when `formatexpr` is empty, which is exactly the "no server"
--      case, so the two compose with no conditional.
--
-- Both run the same `wolf_fmt` code in the same binary. `:WolfFmt` picks
-- between them explicitly for the whole-buffer case.
-- Escaped: `formatprg` is handed to the shell, and a `serverPath` under
-- `C:\Program Files\` or `~/my wolf/` would otherwise split into two words.
vim.bo.formatprg = vim.fn.shellescape(require('wolf.config').server_path()) .. ' fmt -'

-- ---------------------------------------------------------------- folds --

-- Folding is set ONLY when tree-sitter can do it properly. The alternative,
-- `foldmethod=indent`, produces folds at every brace-indented line of a
-- width-100 formatted language and — with the default `foldlevel=0` — greets
-- you with a file collapsed to its `fn` headers. Shipping that as a "default"
-- is worse than shipping no folding, so when there is no parser this option is
-- left exactly as the user found it.
if require('wolf.treesitter').available() then
  vim.wo[0][0].foldmethod = 'expr'
  vim.wo[0][0].foldexpr = 'v:lua.vim.treesitter.foldexpr()'
  -- Opened flat. A fold you did not ask for is a fold you have to open.
  vim.wo[0][0].foldlevel = 99
end
