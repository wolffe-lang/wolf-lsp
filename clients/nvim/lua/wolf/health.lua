--- `:checkhealth wolf` — the deliverable users actually rely on.
---
--- Four things have to line up before a `.lu` buffer shows a diagnostic: a
--- binary, a version, a filetype, and an attached client. When it does not
--- work, the useful output is not "LSP not working" — it is *which one of the
--- four*, and what to type next. So every check below ends in a fix line, and
--- the ones that are expected to fail today say so instead of shouting.
---
--- The one rule this file follows without exception: **it reports, it never
--- repairs.** No parser download, no config rewrite, no `PATH` mutation. A
--- health check that fixes things is a health check you cannot trust to tell
--- you what state you were in.
local M = {}

local health = vim.health
local config = require('wolf.config')

--- How long `wolf lsp` gets to answer `initialize`, in ms.
---
--- 5000 is not arbitrary: it is the cold-start budget report 09 derives from
--- fackr, whose UI thread blocks up to 5 s waiting for a server to become
--- ready. A `wolf lsp` slower than this freezes a real editor, so a health
--- check that waited longer would pass a server users experience as broken.
local INITIALIZE_BUDGET_MS = 5000

--- Frame a JSON-RPC message the way the base protocol requires.
---@param message table
---@return string
local function frame(message)
  local body = vim.json.encode(message)
  return ('Content-Length: %d\r\n\r\n%s'):format(#body, body)
end

--- Drive one `initialize` handshake against `bin` and report what came back.
---
--- The whole session is written to stdin up front — `initialize`,
--- `initialized`, `shutdown`, `exit` — and then stdout is read once. That
--- works because the server answers `initialize` before it needs anything from
--- us, and it means this check needs no event loop, no partial-read state
--- machine, and no way to hang: `vim.system(...):wait(timeout)` is the bound.
---
--- The capabilities sent are Neovim's own (`make_client_capabilities`), not a
--- minimal hand-written set, because the answer this check reports —
--- `positionEncoding` — is *negotiated against them*. A hand-written document
--- would report the encoding some other client would have got.
---@param bin string
---@return table? result, string? err
local function initialize(bin)
  local session = table.concat({
    frame({
      jsonrpc = '2.0',
      id = 1,
      method = 'initialize',
      params = {
        processId = vim.uv.os_getpid(),
        rootUri = vim.NIL,
        capabilities = vim.lsp.protocol.make_client_capabilities(),
        clientInfo = { name = 'wolf.nvim/checkhealth', version = tostring(vim.version()) },
      },
    }),
    frame({ jsonrpc = '2.0', method = 'initialized', params = vim.empty_dict() }),
    frame({ jsonrpc = '2.0', id = 2, method = 'shutdown' }),
    frame({ jsonrpc = '2.0', method = 'exit' }),
  })

  local ok, out = pcall(function()
    return vim.system({ bin, 'lsp' }, { stdin = session, text = true }):wait(INITIALIZE_BUDGET_MS)
  end)
  if not ok then
    return nil, tostring(out)
  end
  if out.signal ~= 0 and out.code ~= 0 and (out.stdout or '') == '' then
    return nil, ('exited %s: %s'):format(out.code, vim.trim(out.stderr or '(no stderr)'))
  end

  -- Scan for the response by JSON body, not by `Content-Length` header.
  --
  -- The obvious implementation — split on `\r\n\r\n` — is wrong here and the
  -- reason is worth writing down: `vim.system(…, { text = true })` normalizes
  -- line endings, so the CR bytes the base protocol mandates are gone from
  -- `out.stdout` by the time this code sees it. A header-based parse would
  -- work against the wire and fail against the API that read it.
  --
  -- `%b{}` matches balanced braces, so each iteration is one complete JSON
  -- object regardless of framing, and the loop tolerates anything the server
  -- sends before its answer (a `window/logMessage`, say).
  for body in (out.stdout or ''):gmatch('(%b{})') do
    local decoded_ok, decoded = pcall(vim.json.decode, body)
    if decoded_ok and type(decoded) == 'table' and decoded.id == 1 and decoded.result then
      return decoded.result, nil
    end
  end
  return nil, 'no `initialize` response on stdout'
end

--- The binary, where it came from, and whether it is the version this plugin
--- was built against.
local function check_binary()
  health.start('wolf binary')

  local configured = config.server_path()
  local bin, source = config.resolve()

  if not bin then
    health.error(
      ('no `%s` on %s'):format(configured, source == 'serverPath' and 'the configured path' or 'PATH'),
      {
        'Install the wolf toolchain and put `wolf` on PATH, or set the path explicitly:',
        "    vim.g.wolf = { serverPath = '/abs/path/to/wolf' }",
        'Nothing else in this plugin needs a binary: `.lu` files still highlight',
        'through syntax/wolf.vim, and nothing errors on startup.',
      }
    )
    return nil
  end

  health.ok(('found `%s` (via %s)'):format(bin, source))

  local pin = require('wolf.pin')
  local out = vim.system({ bin, '--version' }, { text = true }):wait(2000)
  local version = vim.trim((out.stdout or '') .. (out.stderr or ''))
  if out.code ~= 0 or version == '' then
    health.warn(('`%s --version` said nothing (exit %s)'):format(bin, out.code), {
      'That binary is probably not wolf. Check what `serverPath` points at.',
    })
    return bin
  end

  local compat = require('wolf.compat')
  local range = compat.min == compat.max_tested and ('exactly %s'):format(compat.min)
    or ('%s .. %s'):format(compat.min, compat.max_tested)

  if version == pin.version then
    health.ok(('version `%s` — matches the pin this plugin was verified against'):format(version))
    health.info(('declared range: wolf %s (%s %s, verified %s)'):format(
      range,
      compat.client,
      compat.client_version,
      compat.verified
    ))
    return bin
  end

  -- Not an error, and never a refusal (ls07 §3). The plugin declares a RANGE
  -- of wolf versions in `compat.json`; this reports which side of it the
  -- binary is on, names both versions and the upgrade path, and lets the
  -- client attach anyway. An out-of-range server usually mostly works, and
  -- blocking the user's editor is worse than warning them.
  --
  -- Pre-1.0 that range is a PIN RANGE and is one version wide, because
  -- wolf-lang tags no releases and the suite has been run against exactly one
  -- build. `docs/COMPAT.md` states the posture rather than implying a
  -- stability this track cannot provide. The word "unsupported" is deliberately
  -- absent, here and in every other surface: it is a policy nobody set.
  local verdict = require('wolf.version').verdict(version)
  local advice = {
    ('The verified pin is wolf-lang `%s` (`%s`).'):format(pin.commit:sub(1, 7), pin.version),
    ('%s %s declares wolf %s, verified %s.'):format(
      compat.client,
      compat.client_version,
      range,
      compat.verified
    ),
  }
  if verdict.state == 'above' then
    vim.list_extend(advice, {
      ('`%s` is NEWER than any wolf the conformance suite has been run'):format(verdict.found),
      'against. Usually fine — the plugin only sends standard LSP — but nothing',
      'verified this combination. Update the plugin, or report what broke:',
      'https://github.com/wolffe-lang/wolf-lsp/issues',
    })
  elseif verdict.state == 'below' then
    vim.list_extend(advice, {
      ('`%s` is OLDER than the floor this plugin declares.'):format(verdict.found),
      'Update the wolf toolchain, or install a plugin version whose range covers it.',
    })
  else
    vim.list_extend(advice, {
      'That string carries no MAJOR.MINOR.PATCH, so no range comparison was made.',
      'Check what `serverPath` points at — it may not be wolf at all.',
    })
  end

  health.warn(
    ('version `%s`, but this plugin was verified against `%s` (declared range: wolf %s)'):format(
      version,
      pin.version,
      range
    ),
    advice
  )
  return bin
end

--- Does `wolf lsp` come up, and what does it negotiate?
---@param bin string?
local function check_server(bin)
  health.start('wolf lsp')
  if not bin then
    health.warn('skipped — no binary to start', { 'Fix the binary check above first.' })
    return
  end

  local result, err = initialize(bin)
  if not result then
    health.error(('`%s lsp` did not answer `initialize`: %s'):format(bin, err), {
      ('The budget is %d ms (report 09\'s cold-start budget).'):format(INITIALIZE_BUDGET_MS),
      ('Reproduce by hand: printf \'\' | %s lsp'):format(bin),
    })
    return
  end

  local caps = result.capabilities or {}
  health.ok(('answered `initialize` within %d ms'):format(INITIALIZE_BUDGET_MS))
  health.info(('positionEncoding: %s'):format(caps.positionEncoding or 'utf-16 (unstated default)'))

  -- Report what it serves, from the answer rather than from memory. A plugin
  -- that lists capabilities in its docs and a server that serves a different
  -- set is how users learn to distrust both.
  local served = {}
  for field, name in pairs({
    hoverProvider = 'hover',
    documentSymbolProvider = 'documentSymbol',
    documentFormattingProvider = 'formatting',
    codeActionProvider = 'codeAction',
    definitionProvider = 'definition',
    referencesProvider = 'references',
    completionProvider = 'completion',
    renameProvider = 'rename',
  }) do
    if caps[field] then
      table.insert(served, name)
    end
  end
  table.sort(served)
  health.info('serves: ' .. (next(served) and table.concat(served, ', ') or '(nothing)'))
end

--- Filetype detection, asked the way Neovim will answer it.
local function check_filetype()
  health.start('filetype detection')
  for name, want in pairs({
    ['a.lu'] = 'wolf',
    ['a.wolfi'] = 'wolfi',
    ['wolf.pkg'] = 'wolfpkg',
    ['wolf.sum'] = 'wolfsum',
  }) do
    -- `vim.filetype.match` with a bare filename resolves through the same
    -- table `:e` uses, without touching the filesystem or opening a buffer.
    local got = vim.filetype.match({ filename = name })
    if got == want then
      health.ok(('`%s` → %s'):format(name, got))
    else
      health.error(('`%s` → %s (expected %s)'):format(name, tostring(got), want), {
        'ftdetect/wolf.lua was not sourced. Is the plugin on the runtimepath?',
        "    :lua print(vim.inspect(vim.api.nvim_get_runtime_file('ftdetect/wolf.lua', true)))",
      })
    end
  end
end

--- Tree-sitter. Absent is the expected state and is reported as such.
local function check_treesitter()
  health.start('tree-sitter')
  local ts = require('wolf.treesitter')
  if not ts.available(true) then
    health.info('no `wolf` parser installed — expected today; the regex fallback is in use', {
      '`wolffe-lang/tree-sitter-wolf` is scaffold-only (licenses and a README);',
      'the grammar is filled opportunistically between compiler sprints.',
      'syntax/wolf.vim is the real highlighting story until it exists, and it is',
      'derived from the same pinned grammar the parser will be.',
      'Nothing is broken and nothing needs installing.',
    })
    return
  end

  health.ok('`wolf` parser found')
  for _, q in ipairs(ts.queries()) do
    if q.err then
      health.error(('queries/wolf/%s.scm does not compile: %s'):format(q.name, q.err), {
        'The shipped queries predate the grammar. Open an issue against wolf-lsp',
        'with this message — the query files live there, not in the grammar repo.',
      })
    elseif (q.patterns or 0) == 0 then
      health.warn(('queries/wolf/%s.scm has no patterns'):format(q.name), {
        'Placeholder. Filled when tree-sitter-wolf has node names to reference.',
      })
    else
      health.ok(('queries/wolf/%s.scm — %d pattern(s)'):format(q.name, q.patterns))
    end
  end
end

--- Open wolf buffers: attached to what, negotiating what, showing how many.
---
--- Reported over EVERY wolf buffer rather than over "the current buffer", and
--- the reason is not thoroughness — it is that the current-buffer version
--- cannot work. `:checkhealth` opens its own scratch buffer and runs the
--- checks inside it, so `nvim_get_current_buf()` is always the health report
--- and never the `.lu` the user is asking about. A check written that way
--- reports "not a wolf buffer" 100% of the time and looks correct while doing
--- it.
local function check_buffers()
  health.start('wolf buffers')

  local buffers = vim.tbl_filter(function(bufnr)
    return vim.api.nvim_buf_is_loaded(bufnr) and vim.bo[bufnr].filetype == 'wolf'
  end, vim.api.nvim_list_bufs())

  local clients = vim.lsp.get_clients({ name = 'wolf' })

  if #buffers == 0 then
    health.info('no wolf buffer is open — attachment not checked', {
      'Open a `.lu` file and run `:checkhealth wolf` again to see the attached',
      'client, the negotiated encoding and the diagnostic count for it.',
    })
    return
  end

  if #clients == 0 then
    health.error(('no `wolf` client attached (%d wolf buffer(s) open)'):format(#buffers), {
      'Is the config enabled?  :lua print(vim.inspect(vim.lsp.config.wolf))',
      'Enable it:              :lua vim.lsp.enable("wolf")',
      'Then:                   :WolfLspRestart',
      'If the server failed to start, `:WolfLspLog` has the reason.',
    })
    return
  end

  for _, client in ipairs(clients) do
    health.ok(
      ('client %d running (root: %s)'):format(
        client.id,
        client.root_dir or 'none — single-file mode'
      )
    )
    -- The negotiated encoding, read off the live client rather than assumed.
    -- Neovim declares all three with utf-8 first and wolf prefers utf-8, so
    -- this normally says `utf-8` — positions are byte offsets and no
    -- conversion happens on either side. Printing it rather than documenting
    -- it is the point: a change to either side's preference shows up here.
    health.info(('positionEncoding: %s'):format(client.offset_encoding or 'utf-16'))
  end

  for _, bufnr in ipairs(buffers) do
    local name = vim.api.nvim_buf_get_name(bufnr)
    local attached = #vim.lsp.get_clients({ bufnr = bufnr, name = 'wolf' }) > 0
    local line = ('%s — %d diagnostic(s)'):format(
      name ~= '' and vim.fn.fnamemodify(name, ':~:.') or ('[buffer %d]'):format(bufnr),
      #vim.diagnostic.get(bufnr)
    )
    if attached then
      health.ok(line)
    else
      -- A wolf buffer that no client claimed while other buffers have one is
      -- the interesting case: usually a file outside every root the running
      -- clients cover.
      health.warn(line .. ' — no client attached to this buffer', {
        'A wolf client is running but did not claim this file. Usually it lives',
        'outside the workspace root the running client resolved; `:WolfLspRestart`',
        'from this buffer starts one for its own root.',
      })
    end
  end
end

--- Neovim itself.
local function check_nvim()
  health.start('Neovim')
  local wolf = require('wolf')
  if wolf.supported() then
    health.ok(('%s (floor: 0.11)'):format(tostring(vim.version())))
  else
    health.error(('%s is below this plugin\'s 0.11 floor'):format(tostring(vim.version())), {
      '0.11 is where `lsp/<name>.lua` discovery, `vim.lsp.config`/`enable` and',
      'native positionEncoding negotiation landed — the plugin is glue around',
      'those, so there is nothing to back-port.',
      'On 0.10, use nvim-lspconfig instead; the recipe is in `:h wolf-lspconfig`.',
    })
  end
end

function M.check()
  check_nvim()
  local bin = check_binary()
  check_server(bin)
  check_filetype()
  check_treesitter()
  check_buffers()
end

return M
