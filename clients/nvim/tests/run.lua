-- A test runner in forty lines, deliberately.
--
-- The sprint asks for "a busted/plenary test". Neither is a dependency this
-- plugin can honestly take: plenary is a plugin users would have to install to
-- run wolf's tests, busted needs luarocks, and the wolf-lsp repo's whole
-- posture is dependency thinness (ls00 §7 — the JSON-RPC framing is hand-rolled
-- for the same reason). Adding a test framework to a plugin whose entire test
-- surface is "does the filetype resolve, are the commands there, does the
-- health module have the shape it claims" would be more machinery than
-- subject.
--
-- So this is the delta from the sprint text, stated rather than hidden: the
-- assertions the sprint asks for exist and run in CI; the framework it names
-- does not. `tests/plugin_spec.lua` returns a list of `{ name, fn }` and would
-- port to either framework in an afternoon if a real need for one appears.
--
--   nvim --headless -u tests/minimal.lua -l tests/run.lua
--
-- Exits 0 when everything passed, 1 otherwise, and prints one line per test.

local here = vim.fs.dirname(debug.getinfo(1, 'S').source:sub(2))

local passed, failed = 0, 0
local failures = {}

for _, file in ipairs({ 'plugin_spec.lua' }) do
  local chunk = assert(loadfile(vim.fs.joinpath(here, file)))
  for _, case in ipairs(chunk()) do
    -- `xpcall` with a traceback: a failing assertion three calls deep in a
    -- helper is unreadable without one, and this runner has no other
    -- diagnostics to offer.
    local ok, err = xpcall(case.fn, debug.traceback)
    if ok then
      passed = passed + 1
      io.write('ok    ', case.name, '\n')
    else
      failed = failed + 1
      table.insert(failures, { name = case.name, err = err })
      io.write('FAIL  ', case.name, '\n')
    end
  end
end

for _, f in ipairs(failures) do
  io.write('\n--- ', f.name, '\n', tostring(f.err), '\n')
end

io.write(('\n%d passed, %d failed\n'):format(passed, failed))
-- `os.exit` rather than `vim.cmd('cq')`: `-l` scripts run before the UI, and
-- an explicit status is what a CI step reads.
os.exit(failed == 0 and 0 or 1)
