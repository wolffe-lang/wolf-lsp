# facsimile

**Tier 0 — a daily driver.** facsimile (`fac`) is a terminal editor in modern
Fortran; its LSP client is `lsp_server_manager_module` over a hand-rolled
`json_module` and a C `fork`/`execl` process wrapper. It went second, and it
went *because* of what it cannot do: no cancellation, no `\uXXXX` decoding,
`real64` request ids, and an empty `handle_request` stub that answers no
server→client request at all.

That last one is the whole reason this client is in tier 0. A server that
blocks on `workspace/configuration` hangs this editor forever, silently.
Discovering that against a real client, early, is worth more than any amount of
protocol reading.

- Upstream: `FortranGoingOnForty/facsimile`, read at `a121ab3` (trunk, v0.35.0) — re-read at le05 and again at **le07**, where trunk had not moved: `a121ab3` is still the tip. The patch series' base was `1242ffa` (v0.32.8), **38** commits back (`git rev-list --count 1242ffa..a121ab3`)
- Honest characterization: [`CLIENT.md`](CLIENT.md)
- Capability profile: [`profiles/facsimile.json`](../../profiles/facsimile.json)
- Recorded session: [`transcripts/facsimile/smoke.jsonl`](../../transcripts/facsimile/smoke.jsonl)
- Patch series: [`patches/`](patches/)
- Token table provenance: [`inventory.md`](inventory.md)

## Setup

Nothing to install and nothing to configure. `wolf lsp` **is** the compiler
(D34), and facsimile's server table is compiled in:

1. Put `wolf` on `PATH`.
2. Open a `.lu` (or `.wolfi`) file.

```fortran
! src/lsp/lsp_server_manager_module.f90 — get_language_for_file
case('.lu', '.wolfi')
    language = "wolf"

! src/lsp/lsp_server_manager_module.f90 — load_default_configs
call add_config(manager, "wolf", "wolf", "wolf lsp", "*.lu,*.wolfi", caps)
```

The command string is handed to `/bin/sh -c`, so `"wolf lsp"` works verbatim.
There is no LSP config file — the hardcoded table is the design, and adding a
config format to land one server would be scope theft (ls03 non-targets).

**The capability flags no longer drive routing — the server's own reply does.**
This paragraph used to say the opposite, and it was true when written: only
`CAP_HOVER`, `CAP_FORMATTING`, `CAP_DOCUMENT_SYMBOLS` and `CAP_CODE_ACTIONS`
were set, those flags decided where requests went, and an optimistic entry
produced `MethodNotFound` and a user who concluded wolf was broken.

facsimile PR #5 inverted it. `lsp_server_t` now carries nine `supports_*`
fields filled from the `initialize` reply
(`lsp_server_manager_module.f90:64-73`, `:736-783`), and `server_serves()`
(`:1156-1181`) consults them for every key; the static `add_config` table
(`:339-354`) is demoted in its own comment to "the floor used before that
reply arrives", explicitly so that "it can no longer gate off something the
running server does serve, which is what happened to completion at wolf
0.2.1 (facsimile#4)". The wolf row still sets the same four flags, and that is
now harmless rather than load-bearing — the pessimism costs nothing once the
server has spoken.

`CAP_DIAGNOSTICS` is still **not** set, but the old reason for it — "the flag
is read by no routing code anywhere in the editor" — has expired:
`server_serves` special-cases it by name at `:1170`, because diagnostics arrive
as notifications and there is no advertised capability to consult. All nine
capabilities have call sites now (`command_handler_module.f90`, nine
`get_lsp_server_for_cap` sites), so the "only four have call sites" count is
also retired.

## What works

Proven in `transcripts/facsimile/smoke.jsonl`, a real 15-record session:

| feature | key | notes |
|---------|-----|-------|
| diagnostics | — | push, on open and on every 0.5 s-debounced change |
| document symbols | `F4` / `Alt+O` | panel renders `fn main :9` |
| hover | `Ctrl+H` | `who: str`, range correct in UTF-16 units |
| formatting | `Alt+Shift+F` | after this sprint's binding fix — it was dead |
| code actions | `F10` / `Alt+.` | wolf's fix-its are fully resolved |
| syntax highlighting | — | independent of LSP; see `inventory.md` |
| comment toggle | `Ctrl+/` | `//` only — wolf has no block comment form |

## Known limitations — stated honestly

Fully enumerated in [`CLIENT.md`](CLIENT.md). The ones that constrain the
server are in [`docs/SERVER-CONSTRAINTS.md`](../../docs/SERVER-CONSTRAINTS.md).
The short version, none of which is worked around in this repo:

- **No response to any server→client request, ever.** `handle_request` is an
  empty stub. This is the hard constraint on wolf.
- **No `\uXXXX` decoding.** The server must emit raw UTF-8; escapes reach the
  user as literal characters.
- **No `$/cancelRequest`.** In-flight requests are abandoned, never cancelled.
- **No `shutdown`/`exit`.** SIGTERM, then SIGKILL 100 ms later.
- **No `didClose`.** `notify_file_closed` exists, is exported, is imported by
  two modules — and has zero call sites. Documents accumulate for the session.
- **`didChange` version is still 1 on the wire**, so it carries no ordering
  signal — but the shape of that changed at `a121ab3` and le07 re-recorded it.
  `notify_file_changed` now takes an OPTIONAL `version` argument and defaults
  `doc_version = 1` only when it is absent, and `document_sync_module` keeps a
  real counter beside it (`sync%version = sync%version + 1`, incremented on
  every flush). **Neither of the two call sites passes it** —
  `document_sync_module.f90:103` increments the counter on the line above and
  then omits the argument, and `command_handler_module.f90:12612` never had
  one. So the plumbing to fix this exists and is one argument away at each
  site, which is a better bug report than "hardcoded" was.
- **O(n²) JSON**, so large responses visibly stall the editor.
- **No gutter markers.** Diagnostics render in a panel; the gutter code is
  commented out, and `docs/LSP_GUIDE.md` documents behaviour that does not
  exist.

## Recording the transcript

facsimile spawns its server through `/bin/sh -c`, so a proxy named `wolf`
earlier on `PATH` captures everything with no instrumented build:

```sh
# shim/wolf, chmod +x
#!/bin/sh
cd "$WOLF_LSP_ROOT/vendor/upstream/samples" || exit 1
exec "$WOLF_LSP_ROOT/target/debug/lspconf" capture \
  --name facsimile/smoke --profile facsimile --workspace vendor/upstream/samples \
  -- "$WOLF_REAL" lsp
```

The editor itself is driven headlessly through a pty with `pexpect` + `pyte`,
exactly the way facsimile's own `test/integration_*.py` suites drive it. The
session opens `hello.lu`, waits for the clean publish, types a character and
waits out the 0.5 s debounce (E0203 arrives), deletes it and waits again (the
clean publish returns), moves the cursor onto `who`, hovers, requests document
symbols, formats, and quits with `Ctrl+Q`.

The driver script is not committed to either repo: it is scaffolding, and the
transcript is the artifact. The recipe above plus the key sequences in
`patches/STATUS.md` reproduce it.

There is **no `.lsps` beside the transcript**, and that is the point — no
script decided what facsimile sent. `lspconf verify` knows the shape
(`<client>/<scenario>` for a client in `profiles::REAL_CLIENTS`) and
`lspconf replay transcripts/facsimile/smoke.jsonl` runs it against a live
server.

## Verification, and where it lives

Split deliberately, because facsimile's CI cannot run any of it:

- **In wolf-lsp**: the profile validates, the transcript replays (7 matched
  records), `lspconf onetruth` runs all 10 samples **under the facsimile
  profile** as one of five, and `lspconf fuzz --profile=facsimile
  --splices=200` puts a long debounced full-text edit session through this
  client's shape. The raw-UTF-8 constraint is pinned by a test that reads raw
  frame bytes (`tests/encoding.rs`).
- **In facsimile**: `make` (zero warnings), `make check-windows`,
  `make check-render`, `make test-lsp`, and the 132-column limit all pass with
  the patch applied.

## The CI lane facsimile does not get

The sprint offers a `test/integration_wolf.py` pexpect suite, since facsimile's
CI auto-discovers `test/integration_*.py`. **It is deliberately not written.**

`.github/workflows/tests.yml` discovers `test/integration_*.py`, shards them
round-robin, and then fails the shard if **either** a suite exited non-zero
**or** its output contained `SKIP: missing dependency`:

```sh
if [ -n "$fail" ] || [ -n "$skip" ]; then
```

That is a deliberate design choice — a missing dependency must not quietly turn
a suite into a no-op. A wolf suite would need a `wolf` binary on every runner,
and wolf-lang publishes no release artifact and is a private repo, so the file
would turn facsimile's CI red on every push, forever, for a language its
maintainer may not have installed.

The skip detector greps for one exact string, so a suite that printed
`SKIP: no wolf binary` instead would slip through green. That loophole is not
taken here: it games a gate the maintainer built on purpose, and a suite that
silently tests nothing is the failure the gate exists to prevent.

That is a facsimile decision, not wolf's to make, so the lane is described here
and left for its owner. If wolf-lang ever publishes artifacts (ls07), this
becomes cheap and should be revisited.
