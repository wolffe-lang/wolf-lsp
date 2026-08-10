-- The plugin lane: everything assertable WITHOUT a server.
--
-- This is the half of the suite that stays green on a machine with no `wolf`
-- binary — which is every CI runner this repo has, because wolf-lang publishes
-- no release artifact (see the repo README). What it proves is the four things
-- a user's first five minutes depend on: the filetype resolves, the buffer
-- options are the toolchain's, the commands exist, and `:checkhealth wolf`
-- produces every line it promises including the ones that report absence.
--
-- The server-dependent half is `tests/smoke.lua`, which records a real session.

local eq = function(want, got, what)
  assert(
    vim.deep_equal(want, got),
    ('%s: expected %s, got %s'):format(what or 'value', vim.inspect(want), vim.inspect(got))
  )
end

local truthy = function(got, what)
  assert(got, ('%s: expected a truthy value, got %s'):format(what or 'value', vim.inspect(got)))
end

--- A wolf buffer with `text`, never a file on disk.
---
--- ls00 §5: fixtures are requested upstream, never forked locally, and the one
--- recorded exception is `fixtures/astral.lu`. A test that wrote its own `.lu`
--- would be a second exception with no gap entry behind it, so these buffers
--- exist only in memory.
---@param text string
---@return integer bufnr
local function wolf_buffer(text)
  local bufnr = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, vim.split(text, '\n', { plain = true }))
  vim.api.nvim_set_option_value('filetype', 'wolf', { buf = bufnr })
  return bufnr
end

return {
  {
    name = 'filetype: extensions and manifests resolve without touching disk',
    fn = function()
      eq('wolf', vim.filetype.match({ filename = 'a.lu' }), '.lu')
      eq('wolf', vim.filetype.match({ filename = '/deep/path/hello.lu' }), 'nested .lu')
      eq('wolfi', vim.filetype.match({ filename = 'a.wolfi' }), '.wolfi')
      eq('wolfpkg', vim.filetype.match({ filename = 'wolf.pkg' }), 'wolf.pkg')
      eq('wolfsum', vim.filetype.match({ filename = 'wolf.sum' }), 'wolf.sum')
    end,
  },

  {
    -- D35: tier 1 includes windows. A backslash path must resolve the same
    -- way, and `vim.filetype.match` is where that either holds or does not.
    name = 'filetype: a windows-shaped path resolves the same',
    fn = function()
      eq('wolf', vim.filetype.match({ filename = [[C:\src\pkg\hello.lu]] }), 'windows .lu')
      eq('wolfpkg', vim.filetype.match({ filename = [[C:\src\pkg\wolf.pkg]] }), 'windows manifest')
    end,
  },

  {
    name = 'ftplugin: buffer options are the toolchain constants',
    fn = function()
      local bufnr = wolf_buffer('fn main() -> !int {\n    0\n}')
      local get = function(name)
        return vim.api.nvim_get_option_value(name, { buf = bufnr })
      end
      -- `[gram.lex.comment]` — and no block form, so no second commentstring.
      eq('// %s', get('commentstring'), 'commentstring')
      eq(':///,://!,://', get('comments'), 'comments')
      -- wolf_fmt::doc::INDENT
      eq(4, get('shiftwidth'), 'shiftwidth')
      eq(true, get('expandtab'), 'expandtab')
      -- wolf_fmt::doc::WIDTH
      eq(100, get('textwidth'), 'textwidth')
      -- 't' MUST be absent or textwidth hard-wraps code mid-insert.
      assert(not get('formatoptions'):find('t', 1, true), "formatoptions must not contain 't'")
      truthy(get('formatprg'):find('fmt %-$'), 'formatprg ends in `fmt -`')
      -- Left empty on purpose: Neovim installs `vim.lsp.formatexpr()` here on
      -- attach only if it is still empty. Setting it locks the LSP path out.
      eq('', get('formatexpr'), 'formatexpr')
    end,
  },

  {
    name = 'syntax: the regex fallback colours the derived keyword set',
    fn = function()
      vim.cmd('syntax enable')
      local bufnr = wolf_buffer('fn main() -> !int {\n    let who = "wolf"\n    0\n}')
      vim.api.nvim_set_current_buf(bufnr)
      -- Force the syntax file to run for this buffer.
      vim.cmd('doautocmd Syntax')
      eq('wolf', vim.b[bufnr].current_syntax, 'b:current_syntax')

      -- The syntax group ITSELF, not `synIDtrans`: resolving the link chain
      -- collapses `wolfStructure`→`Structure`→`Type`, so a translated
      -- assertion would test Vim's default link table rather than this file.
      local at = function(line, col)
        return vim.fn.synIDattr(vim.fn.synID(line, col, 1), 'name')
      end
      eq('wolfStructure', at(1, 1), '`fn` is a keyword')
      eq('wolfFunction', at(1, 4), '`main` at its definition site')
      eq('wolfStorageClass', at(2, 5), '`let` is a keyword')
      eq('wolfString', at(2, 15), 'the string literal')
      eq('wolfOperator', at(1, 14), '`->`')
    end,
  },

  {
    name = 'commands: every command the sprint names is registered',
    fn = function()
      local commands = vim.api.nvim_get_commands({})
      for _, name in ipairs({
        'WolfFmt',
        'WolfCheck',
        'WolfBuild',
        'WolfRun',
        'WolfLspRestart',
        'WolfLspLog',
      }) do
        truthy(commands[name], ('command :%s'):format(name))
        truthy(commands[name].definition ~= '', ('command :%s has a description'):format(name))
      end
    end,
  },

  {
    name = 'lsp: the config resolves off the runtimepath with the sprint\'s shape',
    fn = function()
      local config = vim.lsp.config.wolf
      truthy(config, 'vim.lsp.config.wolf resolves')
      eq({ 'wolf' }, config.filetypes, 'filetypes')
      eq({ 'wolf.pkg', '.git' }, config.root_markers, 'root_markers, in priority order')
      eq('lsp', config.cmd[2], 'cmd is `<binary> lsp`')
      -- `single_file_support` is not a native field; the equivalent is leaving
      -- `workspace_required` unset, which makes a rootless buffer start a
      -- client anyway. Asserting its ABSENCE is what keeps a well-meaning
      -- future edit from adding a field Neovim ignores.
      eq(nil, config.single_file_support, 'no single_file_support field')
      eq(nil, config.workspace_required, 'workspace_required unset (single-file works)')
      -- The server reads no settings; a settings block here would be inert.
      eq(nil, config.settings, 'no settings block')
    end,
  },

  {
    -- The resolution itself, not just the table. `root_markers` is a LIST IN
    -- PRIORITY ORDER (`:h lsp-root_markers`) — the flat form means "find the
    -- nearest ancestor with wolf.pkg; failing that, the nearest with .git",
    -- while the nested form `{ { 'wolf.pkg', '.git' } }` would make them equal
    -- and let a `.git` in a parent of the package beat the package manifest.
    -- The two forms look nearly identical in a diff and behave differently in
    -- exactly the layout every wolf repo has, so the behaviour is asserted
    -- against a real directory tree.
    name = 'lsp: wolf.pkg outranks .git when both are ancestors',
    fn = function()
      local root = vim.fn.tempname()
      local pkg = vim.fs.joinpath(root, 'mypkg')
      local src = vim.fs.joinpath(pkg, 'src')
      vim.fn.mkdir(src, 'p')
      vim.fn.mkdir(vim.fs.joinpath(root, '.git'), 'p')
      vim.fn.writefile({}, vim.fs.joinpath(pkg, 'wolf.pkg'))

      local file = vim.fs.joinpath(src, 'main.lu')
      vim.fn.writefile({ 'fn main() -> !int { 0 }' }, file)

      local markers = vim.lsp.config.wolf.root_markers
      eq(vim.fs.normalize(pkg), vim.fs.normalize(vim.fs.root(file, markers)), 'wolf.pkg wins')

      -- With no manifest, `.git` is the fallback rather than a failure.
      vim.fn.delete(vim.fs.joinpath(pkg, 'wolf.pkg'))
      eq(vim.fs.normalize(root), vim.fs.normalize(vim.fs.root(file, markers)), '.git is the fallback')

      -- And with neither marker, the search leaves the scratch tree entirely.
      -- Whatever it finds above it (or nothing) is single-file territory, not
      -- an error: `workspace_required` is unset, so `vim.lsp.start` starts the
      -- client with a nil root and a scratch `.lu` still diagnoses.
      --
      -- Asserted as "not inside our tree" rather than as `nil`, because `nil`
      -- is only true on a machine with no `.git` anywhere above the temp
      -- directory — and this one has a `/tmp/.git`. A test that passes here
      -- and fails on a colleague's laptop is worse than no test.
      vim.fn.delete(vim.fs.joinpath(root, '.git'), 'rf')
      local escaped = vim.fs.root(file, markers)
      assert(
        escaped == nil or not vim.startswith(vim.fs.normalize(escaped), vim.fs.normalize(root)),
        ('with no marker the search must leave the tree, got %s'):format(tostring(escaped))
      )
    end,
  },

  {
    name = 'lsp: serverPath override reaches cmd, and nothing else does',
    fn = function()
      local config = require('wolf.config')
      eq({ vim.g.wolf.serverPath, 'lsp' }, config.cmd(), 'cmd honours serverPath')
      eq({ 'wolf', 'lsp' }, { vim.lsp.config.wolf.cmd[1] and 'wolf' or '?', 'lsp' }, 'shape')
    end,
  },

  {
    name = 'treesitter: absence is detected, is not an error, and yields four query slots',
    fn = function()
      local ts = require('wolf.treesitter')
      -- No parser is the expected state at this pin; if one ever IS installed
      -- on a runner this must not turn into a failure, so the assertion is on
      -- the shape of the answer rather than on its value.
      assert(type(ts.available(true)) == 'boolean', 'available() answers a boolean')
      local queries = ts.queries()
      eq(4, #queries, 'four query files are wired')
      local names = vim.tbl_map(function(q)
        return q.name
      end, queries)
      eq({ 'highlights', 'injections', 'folds', 'indents' }, names, 'query names')
      -- Registration happened even with no parser: it is a table entry.
      eq('wolf', vim.treesitter.language.get_lang('wolf'), 'ft wolf -> lang wolf')
    end,
  },

  {
    name = 'quickfix: a diag_schema line becomes a positioned item, verbatim',
    fn = function()
      -- A scratch directory outside the repo entirely, so no `.lu` is authored
      -- inside a tree that has rules about them.
      local dir = vim.fn.tempname()
      vim.fn.mkdir(dir, 'p')
      local body = 'fn main() -> !int {\n    ;\n    0\n}\n'
      local fd = assert(io.open(vim.fs.joinpath(dir, 'probe.lu'), 'wb'))
      fd:write(body)
      fd:close()

      -- Byte 24 is the `;` on line 2 (line 1 is 20 bytes including its \n).
      local line = vim.json.encode({
        diag_schema = 1,
        code = 'E0002',
        severity = 'error',
        message = 'this `;` terminates nothing',
        primary = { file = 0, span = { 24, 25 }, label = 'no statement comes before it' },
        notes = { 'remove the `;`' },
        files = { 'probe.lu' },
      })

      local items = require('wolf.quickfix').items({ line, 'not json at all' }, dir)
      eq(2, #items, 'one diagnostic plus one note')
      eq(2, items[1].lnum, 'line')
      eq(5, items[1].col, 'byte column')
      eq('E', items[1].type, 'severity')
      eq(2, items[1].nr, 'code number')
      -- D22: the compiler's words, unlaundered.
      truthy(items[1].text:find('this `;` terminates nothing', 1, true), 'message verbatim')
      truthy(items[1].text:find('E0002', 1, true), 'code present')
      eq('I', items[2].type, 'the note is informational')

      -- REGRESSION. `files` echoes the path the compiler was HANDED, so it is
      -- absolute when the invocation was. Joining a cwd onto that produces a
      -- file that does not exist, the byte offset fails to resolve, and every
      -- diagnostic lands on 1:1 — which looks like a plausible quickfix list
      -- rather than like a bug. Found by running `:WolfCheck` for real against
      -- the pinned binary, not by reading the schema.
      local absolute = vim.json.encode({
        diag_schema = 1,
        code = 'E0002',
        severity = 'error',
        message = 'this `;` terminates nothing',
        primary = { file = 0, span = { 24, 25 } },
        files = { vim.fs.joinpath(dir, 'probe.lu') },
      })
      local from_abs = require('wolf.quickfix').items({ absolute }, '/somewhere/else')
      eq(2, from_abs[1].lnum, 'an absolute files entry still resolves its line')
      eq(5, from_abs[1].col, 'and its column')
    end,
  },

  {
    name = 'health: every promised section is produced, and absence reads as absence',
    fn = function()
      -- The real thing, not a mock: `:checkhealth wolf` is what a user types,
      -- and the failure this catches (a module that raises halfway through) is
      -- invisible to any test that only requires the module.
      vim.cmd('checkhealth wolf')
      local report = table.concat(vim.api.nvim_buf_get_lines(0, 0, -1, false), '\n')

      for _, section in ipairs({
        'Neovim',
        'wolf binary',
        'wolf lsp',
        'filetype detection',
        'tree-sitter',
        'wolf buffers',
      }) do
        truthy(report:find(section, 1, true), ('health section: %s'):format(section))
      end

      -- The tree-sitter line must be INFO, not a warning: no parser is the
      -- expected state and telling users to fix it would be telling them to
      -- fix something that is not broken.
      truthy(
        report:find('expected today', 1, true) or report:find('parser found', 1, true),
        'tree-sitter absence is stated as expected'
      )
      -- Filetype detection is checked against real matches, so it must pass
      -- even here, where there is no binary at all.
      truthy(report:find('`a.lu` → wolf', 1, true), 'filetype line')
    end,
  },

  {
    -- The sprint's acceptance asks that EVERY failure mode be exercised, not
    -- just that the happy path renders. A health check is only worth anything
    -- in the states where something is wrong, and those are exactly the states
    -- nobody reproduces by hand twice.
    --
    -- Three of the four are here (no binary, wrong version, no attach); the
    -- fourth — no parser — is the ambient state of every runner and is
    -- asserted in the case above.
    name = 'health: no binary, wrong version and no attach each report themselves',
    fn = function()
      local original = vim.g.wolf
      local report = function()
        vim.cmd('checkhealth wolf')
        local text = table.concat(vim.api.nvim_buf_get_lines(0, 0, -1, false), '\n')
        vim.cmd('bwipeout!')
        return text
      end

      -- (1) No binary. An absolute path that cannot exist, so the outcome does
      -- not depend on what happens to be installed on the machine running it.
      vim.g.wolf = { serverPath = vim.fs.joinpath(vim.fn.tempname(), 'no-such-wolf') }
      local missing = report()
      truthy(missing:find('on the configured path', 1, true), 'names the missing binary')
      truthy(missing:find('serverPath', 1, true), 'the fix line names the setting')
      truthy(missing:find('skipped — no binary to start', 1, true), 'the server check stands down')
      -- Nothing else may collapse because the binary is gone.
      truthy(missing:find('`a.lu` → wolf', 1, true), 'filetype detection is unaffected')

      -- (2) Wrong version, from a stand-in that answers `--version` with
      -- something that is not the pin and answers `lsp` with silence. Written
      -- at runtime for both shells rather than committed: a committed fake
      -- binary is a thing that eventually gets run by accident.
      local dir = vim.fn.tempname()
      vim.fn.mkdir(dir, 'p')
      local fake, body
      if vim.fn.has('win32') == 1 then
        fake = vim.fs.joinpath(dir, 'wolf.bat')
        body = {
          '@echo off',
          'if "%1"=="--version" (echo wolf 9.9.9-not-the-pin) else (exit /b 0)',
        }
      else
        fake = vim.fs.joinpath(dir, 'wolf')
        body = {
          '#!/bin/sh',
          'case "$1" in --version) echo "wolf 9.9.9-not-the-pin" ;; *) exit 0 ;; esac',
        }
      end
      vim.fn.writefile(body, fake)
      vim.fn.setfperm(fake, 'rwxr-xr-x')

      vim.g.wolf = { serverPath = fake }
      local stale = report()
      truthy(stale:find('9.9.9-not-the-pin', 1, true), 'reports the version it actually found')
      truthy(stale:find('verified against', 1, true), 'names the pin it expected')
      -- And it must NOT call it unsupported: a version RANGE is ls07's to
      -- define, and this repo's only fact today is one exact pin.
      assert(not stale:find('unsupported', 1, true), 'makes no compatibility claim it cannot back')
      -- A binary that is not a server fails the handshake, on its own line.
      truthy(stale:find('did not answer `initialize`', 1, true), 'the handshake failure is reported')

      -- (3) No attach: a wolf buffer with no client. An error with the three
      -- commands that diagnose it, not silence.
      vim.g.wolf = original
      local bufnr = wolf_buffer('fn main() -> !int {\n    0\n}')
      vim.api.nvim_set_current_buf(bufnr)
      local detached = report()
      truthy(detached:find('no `wolf` client attached', 1, true), 'names the missing attachment')
      truthy(detached:find('WolfLspRestart', 1, true), 'the fix line offers the restart')
      truthy(detached:find('WolfLspLog', 1, true), 'and the log')
    end,
  },

  {
    -- The committed `doc/tags` is what makes `:h wolf-lsp` work for a user who
    -- installed the plugin by unpacking it, with no `:helptags` run. A stale
    -- or missing tags file is invisible until someone asks for help.
    name = 'docs: the committed help tags resolve',
    fn = function()
      for _, tag in ipairs({ 'wolf.nvim', 'wolf-lsp', 'wolf-treesitter', ':WolfFmt' }) do
        local ok, err = pcall(vim.cmd.help, tag)
        assert(ok, ('`:h %s` failed: %s'):format(tag, tostring(err)))
        eq('help', vim.bo.filetype, ('`:h %s` opened a help buffer'):format(tag))
      end
      vim.cmd('helpclose')
    end,
  },

  {
    name = 'docs: the README ships the exact minimal config CI runs',
    fn = function()
      local root = vim.fs.dirname(vim.fs.dirname(debug.getinfo(1, 'S').source:sub(2)))
      local read = function(path)
        local fd = assert(io.open(path, 'rb'), path)
        local text = fd:read('*a')
        fd:close()
        return text
      end
      local minimal = read(vim.fs.joinpath(root, 'tests', 'minimal.lua'))
      local readme = read(vim.fs.joinpath(root, 'README.md'))
      -- Every substantive (non-comment, non-blank) line of the file CI runs
      -- has to appear in the README. A setup guide whose snippet drifts from
      -- the tested one is a guide that stops working without anyone noticing.
      local substantive = 0
      for l in minimal:gmatch('[^\n]+') do
        if not l:match('^%s*%-%-') and l:match('%S') then
          substantive = substantive + 1
          truthy(readme:find(l, 1, true), ('README carries: %s'):format(l))
        end
      end
      assert(substantive >= 2, 'the minimal config should have at least two real lines')
      assert(substantive <= 15, 'the minimal config must stay under 15 lines')
    end,
  },
}
