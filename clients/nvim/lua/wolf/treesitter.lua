--- Tree-sitter wiring — inert today, correct the day the grammar exists.
---
--- `wolffe-lang/tree-sitter-wolf` is **scaffold-only**: at the time this was
--- written the repository contains two licenses and a README and no grammar,
--- and its stated scope is "filled opportunistically between compiler
--- sprints". So this module ships the wiring and none of the pretending:
---
---  * the `wolf` language is registered for the `wolf` filetype, so the moment
---    a `wolf` parser appears on the runtimepath — installed by hand, by
---    `nvim-treesitter`, or bundled by a distribution — it is used;
---  * `queries/wolf/` on this plugin's runtimepath is where the highlight,
---    injection, fold and indent queries live, so query churn is a change to
---    THIS repo and never a grammar release;
---  * absence is detected and is NOT an error. No parser is the expected state
---    today. `syntax/wolf.vim` is the real highlighting story, and it works.
---
--- What this module deliberately does not do is install anything. No download,
--- no `:TSInstall` shellout, no build step. A plugin that compiles a C parser
--- behind your back on first open is a plugin that fails on a machine with no
--- compiler, at the worst possible moment, for a feature that is optional.
local M = {}

--- Is a `wolf` parser loadable?
---
--- `vim.treesitter.language.add` is the only honest test: it does the same
--- `parser/wolf.*` runtimepath search the highlighter will do. Asking
--- `vim.treesitter.language.get_lang` or globbing the runtimepath ourselves
--- would answer a *different* question than the one the highlighter asks.
---
--- Note what it returns, because getting this wrong inverts the check: on
--- success `true`, and on a missing parser `nil, "No parser for language …"` —
--- it does NOT raise. So `pcall` alone always reports success, and the answer
--- is the first return value. `pcall` is still here for the third case: a
--- parser file that exists and fails to load (ABI mismatch, truncated
--- download) DOES raise, and that must read as "absent", not as a stack trace
--- in a user's `.lu` buffer.
---
--- The result is cached for the session. Installing a parser mid-session is a
--- thing that happens exactly once, and `:checkhealth wolf` recomputes.
---@type boolean?
local cached = nil

---@param recheck boolean? Ignore the cached answer (used by the health check).
---@return boolean
function M.available(recheck)
  if cached ~= nil and not recheck then
    return cached
  end
  local ok, added = pcall(vim.treesitter.language.add, 'wolf')
  cached = ok and added == true
  return cached
end

--- Register the language and make this plugin's queries findable.
---
--- Called once from `plugin/wolf.lua`. Safe with no parser installed: the
--- filetype→language mapping is a table entry, not a load.
function M.setup()
  -- Filetype `wolf` → language `wolf`. Explicit even though the names match,
  -- because the identity mapping is not automatic for filetypes Neovim does
  -- not ship, and because `.wolfi` will want the same language under a
  -- different filetype the day the grammar can parse an interface file.
  vim.treesitter.language.register('wolf', { 'wolf' })
end

--- Every query this plugin ships, with whether it currently parses.
---
--- Used only by `:checkhealth`. `vim.treesitter.query.get` raises on a query
--- that names a node the grammar does not have, which is precisely the failure
--- mode a shipped-early query set has — so the check catches it instead of a
--- user's first `.lu` buffer.
---@return { name: string, patterns: integer?, err: string? }[]
function M.queries()
  local out = {}
  for _, name in ipairs({ 'highlights', 'injections', 'folds', 'indents' }) do
    local entry = { name = name }
    if M.available() then
      local ok, query = pcall(vim.treesitter.query.get, 'wolf', name)
      if not ok then
        entry.err = tostring(query)
      elseif query == nil then
        entry.patterns = 0
      else
        -- `query.query:pattern_count()` is the count the runtime actually
        -- compiled, which is the number worth reporting: a file full of
        -- comments reports 0 and says so.
        local counted, count = pcall(function()
          return query.query:pattern_count()
        end)
        entry.patterns = counted and count or 0
      end
    end
    table.insert(out, entry)
  end
  return out
end

return M
