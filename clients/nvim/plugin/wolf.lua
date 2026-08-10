-- Entry point. Everything real is in `lua/wolf/`.
--
-- Filetype detection is NOT here — it is `ftdetect/wolf.lua`, so that a
-- lazy-loaded install (`ft = 'wolf'`) can still detect the filetype that would
-- load it. See the comment in that file.

if vim.g.loaded_wolf then
  return
end
vim.g.loaded_wolf = true

require('wolf').setup()
