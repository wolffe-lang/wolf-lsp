--- Plugin configuration — the whole surface, which is one key.
---
--- `wolf.serverPath` is the only setting this plugin has, and adding a second
--- one needs a reason that survives the question "why is this not a compiler
--- decision?". Wolf's formatter takes no options, its diagnostics are the
--- reviewed artifact (D22), and its server reads no `settings` block; an
--- editor plugin that grew a preferences panel on top of that would be
--- reintroducing exactly the configuration surface the language declined.
---
--- Two ways to set it, and they are the same value:
---
--- ```lua
--- vim.g.wolf = { serverPath = '/path/to/wolf' }   -- before the plugin loads
--- require('wolf').setup({ serverPath = '/path/to/wolf' })
--- ```
---
--- The name is `serverPath` — camelCase, matching the VS Code extension's
--- `wolf.serverPath` (ls05) and the sprint's settings surface — rather than
--- Lua-idiomatic `server_path`, so that one documented name works in every
--- editor. `server_path` is accepted as an alias because half of Neovim will
--- type it anyway, and failing silently on a plausible spelling is a worse
--- trade than an alias.
local M = {}

--- Defaults. Deliberately not a deep table: there is nothing to nest.
local defaults = {
  --- Absolute path to the `wolf` binary, or a bare name to resolve on `PATH`.
  serverPath = 'wolf',
  --- Register the server with `vim.lsp.enable` on load.
  ---
  --- True by default: "install one plugin, open a `.lu`, get diagnostics" is
  --- the deliverable, and a plugin that ships a config it does not enable
  --- makes every user write the same line. Set false and call
  --- `vim.lsp.enable('wolf')` yourself if you want to own that moment.
  autoEnable = true,
}

--- User overrides, in the order they were supplied.
local overrides = {}

--- Normalize the two accepted spellings of every key.
---@param t table?
---@return table
local function canonicalize(t)
  local out = {}
  for k, v in pairs(t or {}) do
    if k == 'server_path' then
      out.serverPath = v
    elseif k == 'auto_enable' then
      out.autoEnable = v
    else
      out[k] = v
    end
  end
  return out
end

--- The resolved settings: defaults < `vim.g.wolf` < `setup()`.
---
--- Read fresh on every call rather than memoized at load. `vim.g.wolf` is a
--- plain global a user can set at any point in their config, including after
--- this module was first required by an `ftdetect` file, and a cached answer
--- would silently serve the pre-config value for the rest of the session.
---@return table
function M.get()
  local g = vim.g.wolf
  return vim.tbl_extend(
    'force',
    defaults,
    canonicalize(type(g) == 'table' and g or nil),
    canonicalize(overrides)
  )
end

--- Record `setup()` arguments. Merged, not replaced: calling `setup` twice
--- (a plugin manager's `opts` plus a hand-written call, which happens) should
--- not lose the first one's keys.
---@param opts table?
function M.merge(opts)
  overrides = vim.tbl_extend('force', overrides, canonicalize(opts))
end

--- The configured binary, unresolved — exactly what the user typed.
---@return string
function M.server_path()
  return M.get().serverPath
end

--- Where the binary actually is, and how it was found.
---
--- Answers the first question `:checkhealth` has to answer and the one a user
--- gets wrong most often: a `serverPath` pointing at a stale build, or a `wolf`
--- earlier on `PATH` than the one their shell finds.
---@return string? path Absolute path, or nil when nothing resolves.
---@return string source `'serverPath'` or `'PATH'`.
function M.resolve()
  local configured = M.server_path()
  local source = configured ~= defaults.serverPath and 'serverPath' or 'PATH'
  -- `exepath` handles both cases: it returns an absolute path for a bare name
  -- found on `PATH`, and for an absolute path it confirms the file exists and
  -- is executable. On Windows it also applies `PATHEXT`, which is why this is
  -- not a hand-rolled `filereadable` check (D35: tier-1 includes win32).
  local resolved = vim.fn.exepath(configured)
  if resolved == nil or resolved == '' then
    return nil, source
  end
  return resolved, source
end

--- The `cmd` a client should be started with.
---@return string[]
function M.cmd()
  return { M.server_path(), 'lsp' }
end

return M
