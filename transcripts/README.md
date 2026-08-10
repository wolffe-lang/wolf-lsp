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

`diagnostics/cross-file-e0302.jsonl` records a **known bug** on purpose
(`divergences.toml`, DIV-LSP-001). The empty diagnostics array in it is the
evidence attached to the filing.

## The format

Frozen at ls00 (§2) and exercised by hand-written fixtures under
`crates/lsp_transcript/tests/fixtures/`. Those fixtures are format exercises,
not sessions, and they never move here.

Client-recorded transcripts land in ls02–ls06, captured from the *actual*
client rather than approximated by a script. Those arrive without a `.lsps`,
and `verify`'s rule will need the corresponding exception when they do.
