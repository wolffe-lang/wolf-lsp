--- wolf.nvim — filetypes, a server config, and six commands.
---
--- The plugin **configures**; it never implements (D34). There is no wolf
--- feature in this directory that the compiler does not already provide: no
--- client-side formatter, no diagnostic post-processing, no reimplementation
--- of anything `wolf lsp` answers. When something is missing from the editor
--- experience, the fix is a compiler sprint, and the honest interim state is
--- documented rather than shimmed.
---
--- Nothing here needs calling. `plugin/wolf.lua` runs `setup()` with no
--- arguments on load; `setup({ … })` is for people who want to pass options in
--- code instead of setting `vim.g.wolf`.
local M = {}

local config = require('wolf.config')

--- Neovim floor. 0.11 is where `lsp/<name>.lua` auto-discovery,
--- `vim.lsp.config`/`vim.lsp.enable` and native `positionEncoding`
--- negotiation all landed — this plugin is a config file plus glue precisely
--- because 0.11 does the rest, and on 0.10 there is no config file to find.
--- Users on 0.10 want the nvim-lspconfig recipe in `:h wolf-lspconfig`.
M.MIN_NVIM = { 0, 11, 0 }

---@return boolean
function M.supported()
  return vim.fn.has('nvim-0.11') == 1
end

--- Format the current buffer.
---
--- Two paths, both `wolf_fmt` inside the same binary:
---
---   1. `textDocument/formatting` when a wolf client is attached and serves it
---      — the response is a full-document edit computed by the compiler.
---   2. `wolf fmt -` over the buffer's bytes when it is not.
---
--- There is deliberately no third path. Nothing in this plugin knows what wolf
--- style *is*, and the day the formatter changes (an edition boundary, D36)
--- nothing here needs to learn.
---@param bufnr integer?
function M.format(bufnr)
  bufnr = bufnr or vim.api.nvim_get_current_buf()

  for _, client in ipairs(vim.lsp.get_clients({ bufnr = bufnr, name = 'wolf' })) do
    if client:supports_method('textDocument/formatting') then
      vim.lsp.buf.format({ bufnr = bufnr, name = 'wolf' })
      return
    end
  end

  local bin = config.resolve()
  if not bin then
    vim.notify('wolf: no binary and no LSP client — nothing can format. :checkhealth wolf', vim.log.levels.ERROR)
    return
  end

  local text = table.concat(vim.api.nvim_buf_get_lines(bufnr, 0, -1, false), '\n')
  -- `wolf fmt` always emits a trailing newline, and the buffer's last line is
  -- one without it. Feeding the buffer back verbatim keeps the input and the
  -- file on disk identical, which matters because the formatter's self-check
  -- compares its own output against its input.
  if vim.bo[bufnr].endofline then
    text = text .. '\n'
  end

  local out = vim.system({ bin, 'fmt', '-' }, { stdin = text, text = true }):wait()
  if out.code ~= 0 then
    -- W0301 (partial format: the file has syntax errors) is a nonzero exit
    -- WITH usable output — error regions pass through byte-identical and clean
    -- siblings still format. Reporting it and applying the result is what the
    -- compiler intends; refusing to apply would make a broken file unformattable.
    vim.notify('wolf fmt: ' .. vim.trim(out.stderr or ''), vim.log.levels.WARN)
    if (out.stdout or '') == '' then
      return
    end
  end

  local formatted = vim.split((out.stdout or ''):gsub('\n$', ''), '\n', { plain = true })
  -- Set only if it changed: an unconditional `nvim_buf_set_lines` bumps
  -- 'modified' and burns an undo state on a no-op format, which is how `gq`
  -- on an already-canonical file starts marking the buffer dirty.
  local current = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  if not vim.deep_equal(current, formatted) then
    local view = vim.fn.winsaveview()
    vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, formatted)
    vim.fn.winrestview(view)
  end
end

--- Stop every wolf client and let them re-attach.
---
--- `vim.lsp.enable(name, false)` stops the clients and drops the activation
--- autocmd; re-enabling re-runs it over already-open buffers. That is the
--- whole restart, and it is why this plugin has no client bookkeeping of its
--- own to get out of sync.
function M.restart()
  vim.lsp.enable('wolf', false)
  vim.lsp.enable('wolf')
end

--- Register the user commands. Idempotent.
local function commands()
  local function cmd(name, fn, opts)
    vim.api.nvim_create_user_command(name, fn, opts or {})
  end

  cmd('WolfFmt', function()
    M.format()
  end, { desc = 'Format the buffer (LSP if attached, `wolf fmt -` otherwise)' })

  cmd('WolfCheck', function()
    local file = vim.api.nvim_buf_get_name(0)
    if file == '' then
      vim.notify('wolf: :WolfCheck needs a file on disk', vim.log.levels.ERROR)
      return
    end
    require('wolf.quickfix').run(
      { 'conform-run', file, '--error-format=json' },
      vim.fs.dirname(file),
      'wolf conform-run'
    )
  end, { desc = 'Diagnose the current file into the quickfix list' })

  -- `wolf build` and `wolf run` land at wolf-lang s31. At this pin the binary
  -- answers every unknown subcommand with `wolf: pre-alpha scaffold; wolf
  -- build|run lands at sprint s31`, so these commands exist, route through the
  -- same renderer as `:WolfCheck`, and surface that sentence verbatim. Two
  -- alternatives were rejected: not shipping them (the sprint names them, and
  -- the day s31 lands nothing would pick them up) and faking them by calling
  -- `conform-run` (a command that silently does something other than what it
  -- says is worse than one that reports it cannot yet).
  cmd('WolfBuild', function()
    require('wolf.quickfix').run({ 'build', '--error-format=json' }, vim.uv.cwd(), 'wolf build')
  end, { desc = 'wolf build into the quickfix list (needs wolf-lang s31)' })

  cmd('WolfRun', function()
    require('wolf.quickfix').run({ 'run', '--error-format=json' }, vim.uv.cwd(), 'wolf run')
  end, { desc = 'wolf run into the quickfix list (needs wolf-lang s31)' })

  cmd('WolfLspRestart', function()
    M.restart()
  end, { desc = 'Stop and re-attach the wolf language server' })

  cmd('WolfLspLog', function()
    vim.cmd.tabnew(vim.lsp.log.get_filename())
  end, { desc = "Open Neovim's LSP log" })
end

--- Wire everything up.
---
--- Deliberately short, and deliberately without a single `autocmd`. Filetype
--- detection is `ftdetect/`, buffer options are `ftplugin/`, server activation
--- is `vim.lsp.enable` — three mechanisms Neovim already owns. A plugin that
--- reimplements any of them with its own autocmd group is a plugin that has to
--- be debugged separately from the editor.
---@param opts table?
function M.setup(opts)
  if opts then
    config.merge(opts)
  end
  if not M.supported() then
    vim.notify(
      ('wolf.nvim needs Neovim 0.11+ (this is %s). See `:h wolf-lspconfig` for the 0.10 recipe.'):format(
        tostring(vim.version())
      ),
      vim.log.levels.WARN
    )
    return
  end

  require('wolf.treesitter').setup()
  commands()

  local settings = config.get()

  -- The only place `serverPath` becomes real. `vim.lsp.config` MERGES over the
  -- table `lsp/wolf.lua` returns, so the upstreamable file stays free of any
  -- `vim.g` reads and this is the one line that knows about the override.
  if settings.serverPath ~= 'wolf' then
    vim.lsp.config('wolf', { cmd = config.cmd() })
  end

  if settings.autoEnable then
    vim.lsp.enable('wolf')
  end
end

return M
