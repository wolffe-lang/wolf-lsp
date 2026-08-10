-- The minimal wolf.nvim configuration, and the one CI runs.
--
-- Two substantive lines, and that is the whole setup. `wolf lsp` is the
-- compiler (D34), so there is no server to install, no `mason` entry, no
-- `capabilities` table to hand around and no `on_attach` to write —
-- `plugin/wolf.lua` calls `require('wolf').setup()` for you once the plugin is
-- on the runtimepath, and Neovim 0.11's `vim.lsp.enable` does the rest.
--
-- `serverPath` is only here because a wolf checkout's binary is not usually on
-- PATH; drop that line entirely once `wolf` is installed.
--
-- Headless: nvim --headless -u clients/nvim/tests/minimal.lua …

vim.opt.runtimepath:prepend(vim.fs.dirname(vim.fs.dirname(debug.getinfo(1, 'S').source:sub(2))))
vim.g.wolf = { serverPath = vim.env.WOLF_BIN or 'wolf' }
