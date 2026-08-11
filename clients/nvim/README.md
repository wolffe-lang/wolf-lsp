# nvim

**Tier 1 — the plugin tier.** Unlike `fackr/` and `facsimile/`, this directory
holds code: `wolf.nvim` is a Neovim plugin and this is its root. It ships from
here until plugin distribution is decided (ls07), so `clients/nvim/` is
simultaneously the deliverable, the mirror and the documentation — which is why
the layout below looks like a plugin rather than like the two tier-0 subtrees.

Neovim is the first client whose support is a thing we *maintain* rather than a
patch we send: the config is upstreamable to `nvim-lspconfig` verbatim, and the
plugin adds the four things a config file cannot (filetypes, buffer options,
highlighting, and a health check that names which of the moving parts is
missing).

- Upstream: `neovim/neovim`, read at `v0.12.4` (`NVIM v0.12.4`, RelWithDebInfo,
  LuaJIT 2.1.1785192264)
- Capability profile: [`profiles/nvim.json`](../../profiles/nvim.json)
- Recorded session: [`transcripts/nvim/smoke.jsonl`](../../transcripts/nvim/smoke.jsonl)
- Token table provenance: [`inventory.md`](inventory.md)
- Tree-sitter query status: [`queries/README.md`](queries/README.md)
- In-editor documentation: `:h wolf.nvim` ([`doc/wolf.txt`](doc/wolf.txt))

## What is in here

```
lsp/wolf.lua              the server config — also the upstreamable lspconfig entry
ftdetect/wolf.lua         .lu .wolfi wolf.pkg wolf.sum
ftplugin/wolf.lua         buffer options, all read off the toolchain
ftplugin/wolfi.lua        the same, for generated interface files
syntax/wolf.vim           regex highlighting, derived from the pinned grammar
syntax/wolfi.vim          sources the above; one file to maintain
queries/wolf/*.scm        tree-sitter queries — deliberately empty, see queries/README.md
plugin/wolf.lua           eleven lines: guard, then require('wolf').setup()
lua/wolf/init.lua         setup, :WolfFmt's two paths, restart
lua/wolf/config.lua       the one setting
lua/wolf/health.lua       :checkhealth wolf
lua/wolf/treesitter.lua   registration, absence detection
lua/wolf/quickfix.lua     diag_schema JSON -> quickfix, one renderer for both surfaces
lua/wolf/pin.lua          GENERATED from vendor/upstream/PIN
doc/wolf.txt              vimdoc, tags committed
tests/                    the plugin lane and the smoke session
```

## Setup

`wolf lsp` **is** the compiler (D34), so there is no server to install and no
version to keep in sync with anything.

1. Put `wolf` on `PATH`.
2. Install the plugin.
3. Open a `.lu` file.

### Installing

The plugin is a **subdirectory** of this repository, and that is the only thing
that makes the recipes differ. lazy.nvim has no option to put a subdirectory of
a repository on the runtimepath, so until `wolf.nvim` ships from its own
repository (ls07), lazy users point at a local checkout:

```lua
-- lazy.nvim
{ dir = '/path/to/wolf-lsp/clients/nvim', ft = { 'wolf', 'wolfi' } }
```

```lua
-- packer.nvim — has a subdirectory option, so it installs from the repository
use { 'wolffe-lang/wolf-lsp', rtp = 'clients/nvim' }
```

```sh
# built-in packages, no plugin manager
git clone https://github.com/wolffe-lang/wolf-lsp \
  ~/.local/share/nvim/site/pack/wolf/start/wolf-lsp
ln -s ~/.local/share/nvim/site/pack/wolf/start/wolf-lsp/clients/nvim \
      ~/.local/share/nvim/site/pack/wolf/start/wolf.nvim
```

### The minimal configuration, which is the one CI runs

Not a simplified illustration of it — [`tests/minimal.lua`](tests/minimal.lua)
is this file, a test asserts these lines appear here, and every headless run in
this repo is `nvim -u` on it:

```lua
vim.opt.runtimepath:prepend(vim.fs.dirname(vim.fs.dirname(debug.getinfo(1, 'S').source:sub(2))))
vim.g.wolf = { serverPath = vim.env.WOLF_BIN or 'wolf' }
```

The first line is only doing what a plugin manager does — putting
`clients/nvim` on the runtimepath, computed relative to the config file so the
checkout can live anywhere. With a plugin manager installed, your entire wolf
configuration is:

```lua
-- nothing
```

There is no `setup()` to call (`plugin/wolf.lua` makes that call), no
`capabilities` table to thread through, and no `on_attach`. `serverPath` is
only needed to point at a binary that is not on `PATH`.

### Without the plugin at all

You do not need it for diagnostics. Copy [`lsp/wolf.lua`](lsp/wolf.lua) to
`~/.config/nvim/lsp/wolf.lua` and add two lines:

```lua
vim.filetype.add({ extension = { lu = 'wolf', wolfi = 'wolfi' } })
vim.lsp.enable('wolf')
```

That file is byte-for-byte what `neovim/nvim-lspconfig` would take: since
lspconfig 2.0 an lspconfig server entry *is* an `lsp/<name>.lua` returning a
`vim.lsp.Config` table, with the `---@brief` block as its generated
documentation. There is no second copy in a second shape to keep in sync.
Opening that PR is ls07's and waits on wolf-lang publishing an installable
release — lspconfig reasonably declines entries for servers nobody can install.

## What works

Proven in `transcripts/nvim/smoke.jsonl`, a real 20-record session, with every
assertion checked as it was recorded:

| feature | how | notes |
|---------|-----|-------|
| diagnostics | — | push, on open and on every 100 ms-debounced change |
| hover | `K` | `who: str`, range exact |
| document symbols | `gO` | `main`, kind 12 |
| formatting | `gq`, `:WolfFmt` | LSP when attached, `wolf fmt -` when not |
| code actions | `gra` | wolf's fix-its arrive fully resolved |
| quickfix from the CLI | `:WolfCheck` | same codes and positions as the LSP path |
| syntax highlighting | — | independent of LSP; see `inventory.md` |
| comment toggle | `gcc` | `//` only — wolf has no block comment form |
| filetype detection | — | `.lu` `.wolfi` `wolf.pkg` `wolf.sum` |

`K`, `gO`, `gra` and `gcc` are Neovim's own defaults, not keymaps this plugin
sets. It sets none.

## The encoding, which is not what the sprint brief assumed

**Neovim and wolf negotiate `utf-8`.** Neovim declares
`general.positionEncodings: ["utf-8", "utf-16", "utf-32"]` — all three, utf-8
first — and wolf prefers utf-8 when it is offered. Every position on this wire
is a byte offset, with no conversion on either side.

"Neovim is a utf-16 client" is true of its internal default and false of what
it negotiates, and the difference matters: the received wisdom would have
predicted utf-16 here and been wrong. `:checkhealth wolf` prints what the live
client settled on rather than what anyone expects.

This also completes the encoding matrix with real clients rather than synthetic
ones. fackr declares `["utf-32"]`, facsimile `["utf-16"]`, Neovim all three
with utf-8 first — so all three of wolf's negotiation branches are now pinned
by a client someone actually types in, and a change to the preference order
cannot pass by agreeing with itself.

## Known limitations — stated honestly

None of these is worked around in this plugin (D22: the editor layer must not
launder what the compiler said).

**`wolf build` and `wolf run` do not exist at this pin.** They land at wolf-lang
s31; the binary answers every unknown subcommand with `wolf: pre-alpha
scaffold; wolf build|run lands at sprint s31`. `:WolfBuild` and `:WolfRun` ship
anyway, route through the same quickfix renderer as `:WolfCheck`, and surface
that sentence verbatim. Not shipping them would mean nothing picks the
subcommands up the day they land; faking them by calling `conform-run` would
mean a command that silently does something other than what it says.

**There is no `errorformat`.** The sprint asks for one "shared with the LSP
path"; wolf's machine-readable output is `--error-format=json` with byte spans,
and an `errorformat` is a scanf dialect for `file:line:col: message` text.
Pointing one at wolf's human output would re-derive structure the JSON already
states, and re-derive it *differently* from how the LSP path derives it — the
exact divergence `lspconf onetruth` exists to catch. `lua/wolf/quickfix.lua` is
the single renderer instead, and the sharing is structural.

**The tree-sitter queries are empty files.** `tree-sitter-wolf` is
scaffold-only, so there are no node names to write patterns against.
`queries/README.md` explains why a guessed `highlights.scm` would be worse than
none. The wiring is real and inert; the regex fallback is the highlighting
story today.

**The syntax file cannot express three real parts of wolf lexical structure**:
expressions inside `{}` interpolations, the identifier/string fusion in
`re"…"`, and raw-string fences balanced by count past three `#`. All three are
arguments for semantic tokens, which are post-v1 compiler work.

**`.wolfi` gets a filetype and highlighting but no server.** `wolf lsp` does
not parse interface files at this pin, and attaching a client that answers
nothing produces a buffer that looks supported and is not.

**The smoke session pins `root_dir` rather than resolving it.** The vendored
samples directory is a package in every sense except that it cannot carry a
`wolf.pkg` — `vendor/upstream/` is a byte-exact snapshot and `cargo xtask
sync-pin` fails on any file not in `samples.toml`. Left alone, the marker
search climbs past it to wolf-lsp's own `.git` and stamps the recording
machine's absolute path into `rootUri`, which makes the transcript replayable
on exactly one computer. Root-marker *priority* is asserted for real against a
scratch tree in `tests/plugin_spec.lua`, so the behaviour is covered even
though the recording does not exercise it.

**The recorded session has no astral-plane text in it.** No corpus sample at
this pin contains a character above the BMP (`vendor/upstream/samples.toml`,
`[gap.astral_plane]`), and the local stopgap `fixtures/astral.lu` lives outside
the capture's workspace, so a session opening it would carry unelidable paths.
Astral positions are covered by `transcripts/encoding/astral-*` under the
synthetic profiles; the nvim transcript's positional claim is byte-exact but
BMP-only.

**No plugin-manager story for lazy.nvim from the repository.** See *Installing*
above. This is a distribution question (ls07), not a plugin defect.

## Recording the transcript

Neovim spawns its server by bare `PATH` lookup (`cmd = { 'wolf', 'lsp' }`), so
a proxy named `wolf` earlier on `PATH` captures everything with no
instrumented build and no plugin change:

```sh
# shim/wolf, chmod +x
#!/bin/sh
cd "$WOLF_LSP_ROOT/vendor/upstream/samples" || exit 1
exec "$WOLF_LSP_ROOT/target/debug/lspconf" capture \
  --name nvim/smoke --profile nvim --workspace vendor/upstream/samples \
  -- "$WOLF_REAL" lsp
```

```sh
# from vendor/upstream/samples, with the shim first on PATH
env PATH="$PWD/../../../shim:$PATH" \
    WOLF_LSP_ROOT=/path/to/wolf-lsp \
    WOLF_REAL=/path/to/wolf \
    WOLF_BIN=wolf \
    nvim --headless \
      -u /path/to/wolf-lsp/clients/nvim/tests/minimal.lua \
      -l /path/to/wolf-lsp/clients/nvim/tests/smoke.lua
```

`WOLF_BIN=wolf` matters: it leaves `serverPath` at its default, so the plugin
does not override `cmd` and `PATH` resolution finds the shim.

[`tests/smoke.lua`](tests/smoke.lua) opens `hello.lu`, waits for the clean
publish, hovers on `who`, requests document symbols, requests formatting and
asserts it returns no edits, inserts a stray `;` and waits out the server's
100 ms debounce for `E0002`, removes it and waits for the clean publish to
return, opens `grammar/semicolon.lu` and asserts the diagnostic lands on the
exact byte column `wolf conform-run` reports, then stops the client so the
recorded tail is a real `shutdown`/`exit` pair.

**Every assertion runs while the session is being recorded.** A transcript of a
broken session is worse than none, because it replays green forever.

Recording it three times in a row produces three byte-identical files, and
getting there found a bug in the harness rather than in the plugin. `lspconf
capture`'s pumps forward a frame before recording it — correctly, so the proxy
never changes a session's timing — which left a window in which the server
could answer a request and the downward pump could record the *response* before
the upward pump recorded the request. The first `nvim/smoke` recording came out
with its `initialize` response at `seq: 1`. It is a race, so it reproduced
intermittently and survived a re-record, which is the worst shape a bug in a
recording tool can have. `crates/lsp_harness/src/capture.rs` now takes an
ordering ticket at frame-READ time and sorts on it, with the inverted case
pinned by a unit test. Every transcript this repo records is affected; the ones
already committed were recorded in the right order and are unchanged.

There is **no `.lsps` beside it**, and that is the point — no script decided
what Neovim sent. `lspconf verify` knows the shape (`<client>/<scenario>` for a
client in `profiles::REAL_CLIENTS`) and `lspconf replay
transcripts/nvim/smoke.jsonl` runs it against a live server.

## Verification, and where it lives

Unlike the tier-0 clients, *all* of it lives here — there is no upstream repo
to split the work with, because the plugin is ours.

- **Plugin lane, no server needed** (`tests/plugin_spec.lua`, 14 cases): the
  four filetypes including a Windows-shaped path (D35), every buffer option
  against the formatter's constants, the syntax file's group assignments,
  command registration, the resolved `vim.lsp.config.wolf` shape *and the
  absence of the two fields a reader expects*, root-marker priority against a
  real directory tree, tree-sitter absence handling, the quickfix byte→column
  conversion, `:checkhealth wolf`'s full output **plus three of its four
  failure modes driven for real** (no binary, wrong version from a stand-in
  written at runtime, no attach — the fourth, no parser, is the ambient state),
  the committed help tags, and this README's minimal config against the file CI
  runs. Green on a machine with no `wolf` binary, which is every CI runner this
  repo has, and run on linux, macOS and windows by the `nvim plugin` job.
- **Server lane** (`tests/smoke.lua`): the 7 assertions above, run live.
- **In the harness**: the profile validates, the transcript replays (9 records
  matched), `lspconf onetruth` runs all 10 samples **under the nvim profile**
  as one of six, and `lspconf fuzz --profile=nvim` puts a long full-text edit
  session through this client's shape.
- **Derivation**: `cargo xtask nvim-check` re-derives the 50-keyword set from
  `vendor/upstream/spec/grammar.ebnf` and fails on any difference with
  `syntax/wolf.vim`, and regenerates `lua/wolf/pin.lua` from
  `vendor/upstream/PIN`.

```sh
cargo xtask nvim-check
nvim --headless -u clients/nvim/tests/minimal.lua -l clients/nvim/tests/run.lua
```

## Why there is no busted or plenary

The sprint asks for "a busted/plenary test". Neither is a dependency this
plugin can honestly take: plenary is a plugin users would have to install to
run wolf's tests, busted needs luarocks, and this repo's whole posture is
dependency thinness (the JSON-RPC framing is hand-rolled for the same reason).
`tests/run.lua` is forty lines and prints one line per case.

That is a delta from the sprint text, recorded here rather than hidden.
`tests/plugin_spec.lua` returns a list of `{ name, fn }` and would port to
either framework in an afternoon if a real need appears.
