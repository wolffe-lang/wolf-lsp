# fackr

**Tier 0 — a daily driver.** fackr is a hand-rolled LSP client in ~3.5k lines
of Rust: `serde_json::Value` throughout, no `lsp-types`, no `tower-lsp`, no
async runtime, one detached thread per server. It went first *because* of that.
A client with no type-level safety net is the harshest honest test of whether
`wolf lsp` is a well-behaved server, and it found things a generated client
never will.

- Upstream: `tenseleyFlow/fackr`, read at `496c7e2` (v1.2.1)
- Capability profile: [`profiles/fackr.json`](../../profiles/fackr.json)
- Recorded session: [`transcripts/fackr/smoke.jsonl`](../../transcripts/fackr/smoke.jsonl)
- Patch series: [`patches/`](patches/)
- Token table provenance: [`inventory.md`](inventory.md)

## Setup

There is nothing to install and nothing to configure. `wolf lsp` **is** the
compiler (D34), and fackr's server table is compiled in:

1. Put `wolf` on `PATH`.
2. Open a `.lu` (or `.wolfi`) file.

That is the whole procedure, and the shortness is the design. fackr has no LSP
config file, reads no environment variables, and offers no path override: it
spawns a bare `Command::new("wolf")` and lets `PATH` decide. Adding a config
format to land one server would be scope theft (ls02 non-targets), so the
constraint is documented here instead of engineered around in the editor.

```rust
// src/lsp/manager.rs — register_default_configs
self.register_config(ServerConfig::new("wolf", "wolf", vec!["wolf", "lsp"]));

// src/lsp/types.rs — detect_language
"lu" | "wolfi" => Some("wolf"),
```

Consequences worth knowing before you file a bug:

- **No version check.** Whatever `wolf` resolves first serves. A stale binary
  produces stale diagnostics with no warning. (`lspconf doctor` is where this
  repo refuses to make that mistake; fackr has no equivalent.)
- **No root markers.** fackr has no concept of one. `rootUri` is the editor's
  workspace root — the directory it was opened in — so `wolf.pkg` and `.git`
  play no part in server startup. Sprint text calling for root markers
  describes a mechanism this client does not have, and inventing one is the
  config layer the non-targets rule out.
- **One server per language, first one that spawns.** No multi-root, no
  per-project servers.

## What works

Wired to keys and proven in `transcripts/fackr/smoke.jsonl`:

| feature | key | notes |
|---------|-----|-------|
| diagnostics | — | push, on open and on every change |
| hover | `F1` | |
| go to definition | `F12` | |
| find references | `Shift+F12` | |
| completion | `Ctrl+N` | |
| rename | `F2` | |
| syntax highlighting | — | independent of LSP; see `inventory.md` |

`documentSymbol`, `formatting` and `codeAction` are *plumbed and parsed*, then
dropped on the floor with a `// TODO` — the requests go out and the responses
are discarded. They appear in the transcript because they really are sent.

## Known limitations — stated honestly

These are true of fackr as patched. None is worked around in this repo (D22:
the editor layer must not launder what the compiler said).

**The message text of a diagnostic is never shown.** fackr renders diagnostics
as a single coloured dot in the gutter at `range.start.line`. Everything wolf
writes — the note, the help, the fix-it — is parsed and then discarded.
`relatedInformation` and `tags` likewise. This is the largest gap between what
`wolf lsp` sends and what a fackr user sees, and it is the highest-value
remaining patch. Until it lands, `wolf lsp` should keep the essential sentence
of a diagnostic in the first line of `message` and keep `range.start.line`
exact.

**Positions are code points, and correct only under UTF-32.** fackr counts
columns in `ropey` chars. It now declares `general.positionEncodings:
["utf-32"]` and wolf answers `utf-32`, which makes that arithmetic *literally
correct* rather than accidentally so — but there is no conversion code, so
against a server that answers `utf-16` (the protocol's mandatory fallback, and
what most servers will say) every column past an astral character is off by
one. The client detects this and records it (`ManagedServer::position_encoding`,
surfaced in `LspClient::server_log()`); it does not fix it. Note the
declaration is utf-32 **alone**: wolf prefers utf-8, then utf-16, then utf-32,
so offering utf-16 as a fallback would get utf-16 and defeat the entire patch.
Report 09's suggested `["utf-32","utf-16"]` does not work against this server.

**No `didSave`.** `document_saved` exists with zero call sites, while the
client advertises `synchronization.didSave: true`. `wolf lsp` must therefore
publish diagnostics on open and change, never only on save.

**Full-text sync, every keystroke.** Changes are found by hashing the buffer on
a ~50 ms tick, and every change sends the whole buffer as a single
`contentChanges: [{text}]`. fackr never reads the server's `textDocumentSync`
(wolf advertises `change: 1`, Full, so they agree by luck rather than
negotiation). ls01's fuzz oracle covers this shape:
`lspconf fuzz --profile=fackr --splices=200`.

**Two ordering hazards the server must survive.** A `didChange` can be sent
while the server is still `Initializing` — only `didOpen` is queued — and
re-opening an already-tracked path early-returns without re-syncing content.

**The UI thread blocks up to 5 s** waiting for a server to become `Ready`
(`send_request`). A cold `wolf lsp` slower than that freezes the editor, which
is where report 09's 5 s cold-start budget comes from.

**Server→client requests are answered but not honoured.**
`workspace/configuration` always replies `[]`, `client/registerCapability` is
acked and discarded, and `workspace/applyEdit` is refused with `-32601`
despite `applyEdit: true` being advertised. There is no channel for
server-side settings at all.

**The server is SIGKILLed microseconds after `exit`.** This is why
`transcripts/fackr/smoke.jsonl` ends at the `shutdown` response: the `exit`
notification is written and the process that would have recorded it is killed
in the same breath. Harmless for wolf (it exits on `exit` anyway), visible in
the artifact, and worth fixing upstream.

## Recording the transcript

The session in `transcripts/fackr/smoke.jsonl` is fackr's real traffic, not a
script's impression of it. Because fackr spawns by bare `PATH` lookup, a proxy
named `wolf` earlier on `PATH` captures everything with no instrumentation
build:

```sh
# shim/wolf, chmod +x
#!/bin/sh
cd "$WOLF_LSP_ROOT" || exit 1
exec ./target/debug/lspconf capture \
  --name fackr/smoke --profile fackr --workspace vendor/upstream/samples \
  -- "$WOLF_REAL" lsp
```

```sh
# from a fackr checkout, with the shim first on PATH
env PATH="$PWD/../shim:$PATH" \
    WOLF_LSP_ROOT=/path/to/wolf-lsp \
    WOLF_REAL=/path/to/wolf \
    FACKR_SMOKE_CORPUS=/path/to/wolf-lsp/vendor/upstream/samples \
    cargo test lsp::smoke_wolf::wolf_lsp_corpus_session
```

The subject is `wolf_lsp_corpus_session` in `src/lsp/smoke_wolf.rs`: it opens
`hello.lu`, waits for the (empty) publish, hovers inside an interpolation,
requests document symbols and formatting, opens `grammar/semicolon.lu`, asserts
`E0002`, sends a full-text edit that clears it, closes, and shuts down.

The transcript has **no `.lsps` beside it**, and that is the point — there is
no script, because no script decides what fackr sends. `lspconf verify` knows
the shape (`<client>/<scenario>` for a client in `profiles::REAL_CLIENTS`) and
`lspconf replay transcripts/fackr/smoke.jsonl` runs it against a live server.

## Verification, and where it lives

fackr has **no tests for its LSP module and CI that only builds on tags**, so
verification is split deliberately:

- **In fackr** (`src/lsp/smoke_wolf.rs`, `process.rs`, `protocol.rs`,
  `highlight.rs`): unit tests for framing and negotiation, plus two live tests
  that drive the real client against a real server. They skip loudly with no
  binary. `wolf_lsp_session` is the one that proves the encoding both ways —
  a diagnostic lands on code-point column 20 (21 would mean UTF-16, 23 bytes),
  and a hover at code-point column 22 resolves the identifier after an emoji
  rather than the space one unit to its left.
- **In wolf-lsp**: the profile validates, the transcript replays, the one-truth
  check runs *under the fackr profile* (`lspconf onetruth`, which now includes
  every derived client profile), and the 200-edit fuzz runs in fackr's shape.

## The CI lane fackr does not have

Offered, not assumed (ls02 §6). fackr's CI builds on tags only, so nothing runs
these tests upstream. A minimal lane would be `cargo build` + `cargo test` on
push and PR. It should **not** include `cargo fmt --check` (the tree is ~8.7k
diff lines away from rustfmt-clean at `496c7e2`) and clippy would need
`continue-on-error` (~60 pre-existing warnings). Adding those gates is a
tree-wide cleanup, which is a fackr decision and not wolf's to make — so the
lane is described here and left for its owner.
