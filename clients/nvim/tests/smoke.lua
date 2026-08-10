-- The headless session `transcripts/nvim/smoke.jsonl` is a recording of.
--
-- Run it under the capture proxy (see `clients/nvim/README.md`, "Recording the
-- transcript"). Every assertion below is checked as the session runs, so a
-- transcript that gets committed is one that also PASSED — a recording of a
-- broken session is worse than none, because it replays green forever.
--
--   nvim --headless -u tests/minimal.lua -l tests/smoke.lua
--
-- Two things this file deliberately does not do:
--
--   * it never asserts diagnostic PROSE. D22 puts diagnostic wording upstream,
--     under review there, and an editor-side test that pins a sentence makes
--     this repo a second approval gate on it. Codes and positions are behaviour
--     and are asserted exactly; the message is checked for presence only.
--   * it never works around anything. If wolf answers late, this fails; the
--     fix is upstream, not a longer timeout with a shrug.

-- ONE deviation from the shipped config, and it is about the recording, not
-- about the plugin.
--
-- `root_markers = { 'wolf.pkg', '.git' }` searches upward from the file. The
-- vendored samples directory is a package in every sense except that it cannot
-- carry a `wolf.pkg` — `vendor/upstream/` is a byte-exact snapshot and
-- `cargo xtask sync-pin` fails on any file that is not in `samples.toml`. So
-- the search would climb past it and match wolf-lsp's OWN `.git`, stamping the
-- recording machine's absolute path into `rootUri` and making the transcript
-- replayable on exactly one computer (the capture proxy elides the workspace
-- directory, not its ancestors).
--
-- Pinning `root_dir` to the workspace reproduces what a real user gets — their
-- package root, found via `wolf.pkg` — without inventing a file inside a
-- snapshot. Root-marker PRIORITY is asserted separately and for real, against
-- a scratch tree, in `tests/plugin_spec.lua`.
vim.lsp.config('wolf', { root_dir = vim.uv.cwd() })

local ok_count, failures = 0, {}

local function check(name, fn)
  local ok, err = xpcall(fn, debug.traceback)
  if ok then
    ok_count = ok_count + 1
    io.write('ok    ', name, '\n')
  else
    table.insert(failures, name)
    io.write('FAIL  ', name, '\n', tostring(err), '\n')
  end
end

local function assert_eq(want, got, what)
  assert(
    vim.deep_equal(want, got),
    ('%s: expected %s, got %s'):format(what, vim.inspect(want), vim.inspect(got))
  )
end

-- Count publishes rather than polling `vim.diagnostic.get`. Polling cannot
-- tell "the server has not answered yet" from "the server answered, and the
-- file is clean" — and the clean publish is exactly what this session has to
-- observe twice.
local publishes = 0
local previous = vim.lsp.handlers['textDocument/publishDiagnostics']
vim.lsp.handlers['textDocument/publishDiagnostics'] = function(...)
  publishes = publishes + 1
  return previous(...)
end

--- Block until `publishes` reaches `n`, or fail.
local function wait_publishes(n)
  assert(
    vim.wait(10000, function()
      return publishes >= n
    end, 20),
    ('timed out waiting for publish #%d (saw %d)'):format(n, publishes)
  )
end

local function wolf_client(bufnr)
  local clients = vim.lsp.get_clients({ bufnr = bufnr, name = 'wolf' })
  assert(#clients == 1, ('expected exactly one wolf client, got %d'):format(#clients))
  return clients[1]
end

--- Open a workspace file and wait for the client to attach to it.
local function open(path)
  vim.cmd.edit(path)
  local bufnr = vim.api.nvim_get_current_buf()
  assert(
    vim.wait(15000, function()
      return #vim.lsp.get_clients({ bufnr = bufnr, name = 'wolf' }) > 0
    end, 20),
    ('no wolf client attached to %s'):format(path)
  )
  return bufnr
end

-- ============================================================ the session --

local hello = open('hello.lu')

check('the client attaches and negotiates an encoding', function()
  local client = wolf_client(hello)
  -- THE fact this whole exercise exists to establish, and it is not the one
  -- the sprint brief assumed. Neovim declares `positionEncodings` as
  -- `{ 'utf-8', 'utf-16', 'utf-32' }` — all three, utf-8 FIRST — and wolf
  -- prefers utf-8 when offered. So the negotiated encoding is utf-8 and every
  -- position on this wire is a byte offset, with no conversion on either side.
  -- "nvim is utf-16" is true of Neovim's INTERNAL default and false of what it
  -- negotiates.
  assert_eq('utf-8', client.offset_encoding, 'negotiated positionEncoding')
  -- Single-file support with no `workspace_required`: the sample lives under
  -- a directory with no `wolf.pkg`, and the client started anyway.
  assert(client.initialized, 'client initialized')
end)

check('a clean file publishes zero diagnostics', function()
  wait_publishes(1)
  assert_eq({}, vim.diagnostic.get(hello), 'diagnostics for hello.lu')
end)

check('hover answers with the type and an exact range', function()
  local client = wolf_client(hello)
  -- Line 10, column 10 (1-based) — the `who` binding. Written as LSP
  -- coordinates directly rather than through `make_position_params`, so the
  -- assertion is about the SERVER's answer and not about the cursor.
  local result = client:request_sync('textDocument/hover', {
    textDocument = vim.lsp.util.make_text_document_params(hello),
    position = { line = 9, character = 9 },
  }, 5000, hello)
  assert(result and result.result, 'hover returned nothing')
  local hover = result.result
  assert(hover.contents.value:find('who', 1, true), 'hover names the binding')
  assert(hover.contents.value:find('str', 1, true), 'hover names the type')
  -- The range is behaviour: `who` occupies characters 8..11 of line 9. Under
  -- utf-8 those are byte offsets, and the line is ASCII, so they are also the
  -- column numbers a user sees.
  assert_eq({ line = 9, character = 8 }, hover.range.start, 'hover range start')
  assert_eq({ line = 9, character = 11 }, hover.range['end'], 'hover range end')
end)

check('documentSymbol returns the program\'s one function', function()
  local client = wolf_client(hello)
  local result = client:request_sync('textDocument/documentSymbol', {
    textDocument = vim.lsp.util.make_text_document_params(hello),
  }, 5000, hello)
  assert(result and result.result, 'documentSymbol returned nothing')
  local names = vim.tbl_map(function(s)
    return s.name
  end, result.result)
  assert(vim.tbl_contains(names, 'main'), 'documentSymbol lists `main`, got ' .. vim.inspect(names))
end)

check('formatting a canonical file changes nothing', function()
  local client = wolf_client(hello)
  -- Corpus bytes ARE canonical `wolf fmt` output at STYLE_VERSION 1, so a
  -- formatting response with edits in it would mean the formatter disagrees
  -- with the corpus — a real finding, not a test to relax.
  local result = client:request_sync('textDocument/formatting', {
    textDocument = vim.lsp.util.make_text_document_params(hello),
    options = { tabSize = 4, insertSpaces = true },
  }, 5000, hello)
  assert(result, 'formatting returned nothing')
  assert_eq({}, result.result or {}, 'formatting edits on canonical bytes')
end)

check('an edit produces a diagnostic, and undoing it clears one', function()
  local before = publishes
  -- Break the file: an empty statement, which is E0002 (`[gram.lex.newline]`).
  vim.api.nvim_buf_set_lines(hello, 11, 11, false, { '    ;' })
  wait_publishes(before + 1)
  local diags = vim.diagnostic.get(hello)
  assert(#diags > 0, 'the edited buffer publishes at least one diagnostic')
  assert_eq('E0002', diags[1].code, 'diagnostic code after the edit')
  assert_eq(11, diags[1].lnum, 'the diagnostic lands on the inserted line')
  -- The prose is not asserted (D22), only that there IS some.
  assert(#(diags[1].message or '') > 0, 'the diagnostic carries a message')

  before = publishes
  vim.api.nvim_buf_set_lines(hello, 11, 12, false, {})
  wait_publishes(before + 1)
  assert(
    vim.wait(5000, function()
      return #vim.diagnostic.get(hello) == 0
    end, 20),
    'undoing the edit did not clear the diagnostic'
  )
end)

check('a broken sample lands its diagnostic on the exact byte column', function()
  local before = publishes
  local semicolon = open('grammar/semicolon.lu')
  wait_publishes(before + 1)
  local diags = vim.diagnostic.get(semicolon)
  assert(#diags > 0, 'grammar/semicolon.lu publishes a diagnostic')
  local d = diags[1]
  assert_eq('E0002', d.code, 'code')
  -- `wolf conform-run grammar/semicolon.lu --error-format=json` puts this
  -- span at bytes [222,223]. Byte 222 is line 8, byte column 5 (1-based), so
  -- 0-based that is line 7, character 4. This is the one-truth claim made from
  -- inside the editor: the position the compiler computed is the position the
  -- buffer shows.
  assert_eq(7, d.lnum, 'diagnostic line (0-based)')
  assert_eq(4, d.col, 'diagnostic column (0-based, bytes under utf-8)')
  assert_eq(vim.diagnostic.severity.ERROR, d.severity, 'severity')
end)

-- =============================================================== teardown --

io.write(('\n%d passed, %d failed\n'):format(ok_count, #failures))
for _, name in ipairs(failures) do
  io.write('  failed: ', name, '\n')
end

-- Shut the client down through Neovim's own path, so the recorded tail is the
-- `shutdown`/`exit` pair a user's `:qa` produces. That tail is part of what
-- the transcript is FOR: a client's shutdown behaviour is a server constraint,
-- and this is where it gets recorded rather than assumed.
--
-- `vim.cmd('qall!')` would do the same thing and then exit 0 unconditionally,
-- which would hand CI a green run for a failed session. So the clients are
-- stopped explicitly and the status is ours to set.
for _, client in ipairs(vim.lsp.get_clients({ name = 'wolf' })) do
  client:stop(false)
end
vim.wait(5000, function()
  return #vim.lsp.get_clients({ name = 'wolf' }) == 0
end, 20)
-- The proxy needs a beat to write the last frames it forwarded.
vim.wait(200)

os.exit(#failures == 0 and 0 or 1)
