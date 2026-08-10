--- The command-line surface, rendered into the quickfix list.
---
--- This is the editor-side corollary of `lspconf onetruth`: the LSP path and
--- the command-line path must show the same codes at the same places, because
--- they are the same compiler (D34). Here that is enforced structurally rather
--- than hoped for — there is exactly ONE renderer, and both `:WolfCheck` and
--- `:WolfBuild` feed it.
---
--- ## Why not 'errorformat'
---
--- The sprint asks for "errorformat shared with the LSP path", and the honest
--- implementation of that intent is not an `errorformat` at all. Wolf's
--- machine-readable diagnostics are `--error-format=json`: one `diag_schema: 1`
--- object per diagnostic, carrying **byte spans** and a `files` table, on
--- stderr. An `errorformat` is a scanf dialect for `file:line:col: message`
--- text, and pointing one at wolf's human output would mean re-deriving
--- structure the JSON already states — and re-deriving it *differently* from
--- how the LSP path derives it, which is the exact divergence the one-truth
--- check exists to catch. So: parse the schema, convert byte offsets with the
--- buffer's own bytes, and hand Neovim finished items.
---
--- ## The wording is not touched
---
--- D22: diagnostics are the reviewed artifact. `message` is copied verbatim,
--- notes are appended as their own quickfix lines under the diagnostic they
--- belong to, and nothing here rewords, truncates, or re-orders. A quickfix
--- list that says something the compiler did not say is a bug in this file.
local M = {}

--- Byte offset → (1-based line, 1-based byte column) for a file's bytes.
---
--- Quickfix columns are byte indices when `col` is set without `vcol`, which
--- is exactly what a byte span already is — so this conversion is the only one
--- in the plugin, and there is no encoding question in it at all. (The
--- encoding question lives entirely on the LSP path, where Neovim and the
--- server negotiate it; see `:h wolf-encoding`.)
---@param text string
---@return fun(offset: integer): integer, integer
local function offset_mapper(text)
  -- Line starts, in bytes. Built once per file rather than per diagnostic: a
  -- package with a hundred errors would otherwise rescan the file a hundred
  -- times.
  local starts = { 0 }
  local i = 1
  while true do
    local nl = text:find('\n', i, true)
    if not nl then
      break
    end
    table.insert(starts, nl)
    i = nl + 1
  end
  return function(offset)
    -- Binary search for the last line start at or before `offset`.
    local lo, hi = 1, #starts
    while lo < hi do
      local mid = math.floor((lo + hi + 1) / 2)
      if starts[mid] <= offset then
        lo = mid
      else
        hi = mid - 1
      end
    end
    return lo, offset - starts[lo] + 1
  end
end

--- Is `path` already absolute?
---
--- `files` in a `diag_schema` object ECHOES the path the compiler was handed,
--- so it is absolute when the invocation was absolute and relative when it was
--- not. Joining a cwd onto an absolute path produces a file that does not
--- exist, every byte offset then fails to resolve, and every diagnostic lands
--- on line 1 column 1 — which looks like a plausible quickfix list rather than
--- like a bug. (It was one, found by running `:WolfCheck` for real.)
---
--- Hand-rolled rather than `vim.fs.abspath`, which resolves against the
--- PROCESS cwd; the relevant base here is the directory the compiler ran in.
--- Windows forms are covered because tier 1 includes win32 (D35): a drive
--- letter, and a UNC path.
---@param path string
---@return boolean
local function is_absolute(path)
  return path:sub(1, 1) == '/'
    or path:match('^%a:[/\\]') ~= nil
    or path:sub(1, 2) == '\\\\'
end

--- Read a file's bytes, or nil.
---@param path string
---@return string?
local function read(path)
  local fd = io.open(path, 'rb')
  if not fd then
    return nil
  end
  local text = fd:read('*a')
  fd:close()
  return text
end

--- Turn `diag_schema: 1` lines into quickfix items.
---
--- Non-schema lines are ignored, not an error: the conformance runner also
--- emits a summary object on the same stream, and a future field or a stray
--- warning must not take the whole list down.
---@param lines string[] Raw output lines (stdout and stderr, in any order).
---@param cwd string Directory the `files` table's relative paths resolve against.
---@return table[] items, string[] skipped
function M.items(lines, cwd)
  local items = {}
  local skipped = {}
  -- One mapper per file, shared across every diagnostic that lands in it.
  local mappers = {}

  local function mapper_for(path)
    if mappers[path] == nil then
      local text = read(path)
      mappers[path] = text and offset_mapper(text) or false
    end
    return mappers[path] or nil
  end

  for _, line in ipairs(lines) do
    if line:find('"diag_schema"', 1, true) then
      local ok, d = pcall(vim.json.decode, line)
      if not ok or type(d) ~= 'table' or d.diag_schema == nil then
        table.insert(skipped, line)
      else
        local files = d.files or {}
        local primary = d.primary or {}
        -- `file` is an index into `files`, 0-based (SourceMap intern order).
        local rel = files[(primary.file or 0) + 1]
        local path = ''
        if rel then
          path = vim.fs.normalize(is_absolute(rel) and rel or vim.fs.joinpath(cwd, rel))
        end
        local lnum, col = 1, 1
        local map = path ~= '' and mapper_for(path) or nil
        if map and primary.span then
          lnum, col = map(primary.span[1])
        end
        table.insert(items, {
          filename = path,
          lnum = lnum,
          col = col,
          -- `E`/`W` drives the quickfix sign and `:cc` colouring. Anything
          -- that is not an error is a warning; wolf has no third severity at
          -- this pin, and inventing a mapping for one it might grow is how
          -- the two surfaces drift.
          type = d.severity == 'error' and 'E' or 'W',
          nr = tonumber((d.code or ''):match('%d+')) or 0,
          -- Verbatim. The label is the compiler's own words for this span.
          text = string.format(
            '%s: %s%s',
            d.code or '?',
            d.message or '',
            primary.label and (' (' .. primary.label .. ')') or ''
          ),
          valid = 1,
        })
        for _, note in ipairs(d.notes or {}) do
          table.insert(items, {
            filename = path,
            lnum = lnum,
            col = col,
            type = 'I',
            text = 'note: ' .. note,
            valid = 1,
          })
        end
      end
    end
  end
  return items, skipped
end

--- Run a wolf subcommand over `path` and fill the quickfix list.
---
--- Synchronous on purpose. `:WolfCheck` is a thing you type and then read the
--- answer to; an async version would need a progress UI, a cancellation story
--- and a "which run is this" question, all to save the user from a compiler
--- that answers in milliseconds on the files an editor has open.
---@param args string[] Arguments after the binary, e.g. `{ 'conform-run', f }`.
---@param cwd string
---@param title string
function M.run(args, cwd, title)
  local config = require('wolf.config')
  local bin, source = config.resolve()
  if not bin then
    vim.notify(
      ('wolf: no `%s` binary found (%s). :checkhealth wolf'):format(config.server_path(), source),
      vim.log.levels.ERROR
    )
    return
  end

  local cmd = vim.list_extend({ bin }, args)
  local out = vim.system(cmd, { cwd = cwd, text = true }):wait()

  local lines = {}
  for _, stream in ipairs({ out.stdout or '', out.stderr or '' }) do
    for line in stream:gmatch('[^\n]+') do
      table.insert(lines, line)
    end
  end

  local items = M.items(lines, cwd)
  vim.fn.setqflist({}, ' ', { title = title, items = items })

  if #items == 0 then
    -- A non-zero exit with no parseable diagnostic is the case worth being
    -- loud about: at this pin `wolf build` is still the pre-alpha scaffold
    -- (`wolf build|run lands at sprint s31`) and prints exactly that. Swallow
    -- it and the user sees an empty quickfix list and concludes their code is
    -- fine.
    if out.code ~= 0 then
      vim.notify(
        ('wolf %s exited %d: %s'):format(
          args[1] or '',
          out.code,
          vim.trim((out.stderr or '') .. (out.stdout or ''))
        ),
        vim.log.levels.WARN
      )
    else
      vim.notify('wolf: no diagnostics', vim.log.levels.INFO)
    end
    return
  end
  vim.cmd('copen')
end

return M
