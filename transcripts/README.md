# Transcripts

Recorded JSON-RPC sessions, replayed against the real `wolf lsp` by
`lspconf replay`. One directory per concern, and beside every `x.jsonl` the
`x.lsps` **script** that produced it.

## The script is the input; the transcript is output

A transcript is derived. When the server legitimately changes what it answers,
the fix is `lspconf record` and a reviewed diff — never a hand edit. That only
works if the session itself is committed, so `lspconf verify` fails a
transcript with no script beside it: an un-re-recordable transcript is a golden
byte file, which is the artifact this whole design exists not to be.

Re-record everything: `lspconf rerecord`. One script: `lspconf record <path>`.

## Why there are no timings in these files

The format carries a `t_us` sidecar and `lspconf record --timings` writes it,
but the committed library does not have it. A transcript whose every line
changes on every re-record is a transcript nobody reviews, and the design rests
on that review. Numbers that vary per run belong in `lspconf bench`'s D5 JSONL,
which is where they are.

## The set

- `lifecycle/` — `initialize` under every profile, all three `positionEncoding`
  negotiations plus the unknown-encoding fallback, unimplemented methods,
  shutdown/exit.
- `diagnostics/` — a clean file, broken files with exact E-codes, unsaved
  buffer overlays, and what `didClose` clears.
- `requests/` — hover, `documentSymbol` nesting, formatting byte-stability in
  both directions, and the code-action quickfix round-trip.
- `cancel/` — `$/cancelRequest` honored into the query layer, superseded
  requests, and a cancel for an id nobody is waiting on.
- `encoding/` — the astral fixture under each negotiated encoding, and CRLF as
  `didChange` content. The three `astral-*` files are **not copies**: the same
  semantic targets sit at different `character` numbers in each, and a shim
  that treated all encodings alike would produce three identical transcripts.

`diagnostics/cross-file-e0302.jsonl` used to record a **known bug** on purpose
(`divergences.toml`, DIV-LSP-001): the E0302 whose primary span lands in
`twice.lu` reached no document at all, and the empty diagnostics array was the
evidence attached to the filing. wolf-lang `7117882` fixed it — diagnostics now
publish per primary-span file — so re-recording at pin `70bdd35` turned that
empty array into a real `publishDiagnostics` against `twice.lu`, and the ledger
entry was deleted. The transcript is now the *regression* test for the fix
rather than the evidence for the bug, which is why it was worth recording.

## The format

Frozen at ls00 (§2) and exercised by hand-written fixtures under
`crates/lsp_transcript/tests/fixtures/`. Those fixtures are format exercises,
not sessions, and they never move here.

## Client-recorded transcripts

`fackr/`, `facsimile/` and `nvim/` are these, and they are a different kind of
artifact: captured from the *actual* editor by `lspconf capture` — a proxy the
editor spawns instead of the server — rather than approximated by a script.
There is no `.lsps` beside one, and there cannot be: the whole claim of the
file is that no script decided what the client sent.

No editor needed an instrumented build. All three spawn their server by bare
name (fackr via `Command::new("wolf")`, facsimile via `/bin/sh -c`, Neovim via
`cmd = { 'wolf', 'lsp' }`), so a proxy named `wolf` earlier on `PATH` sees
everything. facsimile is additionally driven through a pty with `pexpect` +
`pyte` — the way its own integration suites drive it — so
`facsimile/smoke.jsonl` records real keystrokes reaching a real editor, 0.5 s
debounce and all. Neovim is driven by a Lua script through `nvim --headless
-l`, its own supported scripting entry point rather than a terminal puppet, and
every assertion in that script runs *while the session is being recorded* — a
transcript of a broken session would replay green forever.

`verify` takes them on exactly that basis. The exemption is keyed on the first
segment of the transcript's own `name` being a client in
`lsp_harness::profiles::REAL_CLIENTS`, **not** on where the file sits — a
transcript cannot buy its way out of the script rule by being filed in a
flattering directory. The re-record path is "drive that editor again", written
down per client under `clients/<client>/`.

Everything else still applies: canonical form, a gapless `seq`, the pin, and a
`profiles/<client>.json` derived from the same session. `replay` runs them
against a live server like any other transcript.
