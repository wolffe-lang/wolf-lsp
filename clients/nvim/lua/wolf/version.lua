--- The compatibility verdict: is the `wolf` on this machine inside the range
--- this plugin declares (ls07 §3)?
---
--- Separated from `health.lua` because it is the only part of the health check
--- that is pure — a string in, a verdict out — and therefore the only part a
--- test can drive without spawning anything. `tests/plugin_spec.lua` calls it
--- directly with hand-written version strings, including the two that matter
--- and are otherwise impossible to produce: a wolf older than `min`, and a wolf
--- newer than anything the suite has run against.
---
--- The range itself lives in `lua/wolf/compat.lua`, which is GENERATED from
--- `clients/nvim/compat.json` by `cargo xtask compat-generate`. Nothing here
--- decides what is supported; it only compares.
local M = {}

--- `wolf 0.0.1 (pre-alpha)` -> `{ 0, 0, 1 }`.
---
--- Returns nil for anything without a bare `MAJOR.MINOR.PATCH` word in it,
--- which is a real case rather than a defensive one: a user's `serverPath` can
--- point at any executable, and "this does not look like a wolf version" is a
--- more useful thing to say than a fabricated comparison.
---@param version_string string
---@return integer[]?
function M.parse(version_string)
  for word in tostring(version_string):gmatch('%S+') do
    local major, minor, patch = word:match('^(%d+)%.(%d+)%.(%d+)$')
    if major then
      return { tonumber(major), tonumber(minor), tonumber(patch) }
    end
  end
  return nil
end

--- Numeric triple comparison. -1, 0 or 1.
---
--- Numeric and not lexical, because `0.10.0 < 0.9.0` as strings and a version
--- check that gets that backwards warns on exactly the upgrades it should not.
---@param a integer[]
---@param b integer[]
---@return integer
function M.cmp(a, b)
  for i = 1, 3 do
    if a[i] ~= b[i] then
      return a[i] < b[i] and -1 or 1
    end
  end
  return 0
end

--- Where `version_string` sits relative to the declared range.
---
--- `state` is one of:
---   'in-range'    inside [min, max_tested]        — say nothing
---   'below'       older than min                  — warn, name the floor
---   'above'       newer than max_tested           — warn, name what is untested
---   'unparseable' no MAJOR.MINOR.PATCH in it      — warn, name what was found
---
--- Never 'unsupported'. The plugin makes no claim it cannot back, and it never
--- refuses to attach: an out-of-range server usually mostly works, and blocking
--- the user is worse than warning them (docs/COMPAT.md).
---@param version_string string
---@return { state: string, found: string?, min: string, max_tested: string }
function M.verdict(version_string)
  local compat = require('wolf.compat')
  local found = M.parse(version_string)
  if not found then
    return { state = 'unparseable', min = compat.min, max_tested = compat.max_tested }
  end
  local text = ('%d.%d.%d'):format(found[1], found[2], found[3])
  local state = 'in-range'
  if M.cmp(found, M.parse(compat.min)) < 0 then
    state = 'below'
  elseif M.cmp(found, M.parse(compat.max_tested)) > 0 then
    state = 'above'
  end
  return { state = state, found = text, min = compat.min, max_tested = compat.max_tested }
end

return M
