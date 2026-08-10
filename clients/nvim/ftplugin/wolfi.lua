-- `.wolfi` — a generated interface file.
--
-- Same lexical surface as `.lu` (it is wolf declarations), so the comment and
-- indent settings are the source ones. What differs is that nobody should be
-- typing in here: the file is produced by the compiler, and an edit is lost on
-- the next build.
--
-- It is NOT marked `readonly` or `nomodifiable`. Neovim has no way to say "the
-- build will overwrite this" and the two options it does have both lie: a user
-- with a good reason to patch a generated file would have to fight the
-- filetype plugin to do it. The honest mechanism is the message below, once,
-- and then getting out of the way.

if vim.b.did_ftplugin then
  return
end
vim.b.did_ftplugin = true

vim.b.undo_ftplugin = 'setlocal comments< commentstring< '
  .. 'expandtab< shiftwidth< softtabstop< tabstop< textwidth< formatoptions< iskeyword<'

vim.bo.commentstring = '// %s'
vim.bo.comments = ':///,://!,://'
vim.bo.expandtab = true
vim.bo.shiftwidth = 4
vim.bo.softtabstop = 4
vim.bo.tabstop = 4
vim.bo.textwidth = 100
vim.bo.formatoptions = 'croqlj'
vim.bo.iskeyword = '@,48-57,_,192-255'

-- No `formatprg`: `wolf fmt` formats source, and a generated interface's bytes
-- are the generator's business.
--
-- No LSP either — `lsp/wolf.lua` lists `filetypes = { 'wolf' }` and `wolfi` is
-- deliberately not in it. `wolf lsp` at this pin does not serve interface
-- files, and attaching a client that will answer nothing produces a buffer
-- that looks supported and is not.
