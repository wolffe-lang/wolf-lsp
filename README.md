# wolf-lsp

The wolf editor layer: clients, configs, conformance testing, and packaging
for the wolf language server.

**The engine is not here.** The language server is `wolf lsp` — the wolf
compiler itself, serving the Language Server Protocol from the same code
that compiles (one process, one truth; see wolf-lang's `wolf_query`
contract). This repo makes editors speak to it:

- the protocol **conformance harness** (recorded JSON-RPC session replay
  against `wolf lsp`, capability snapshots, latency budgets)
- first-class clients: **fackr**, **facsimile**, Neovim, VS Code
- config tier: Helix, Zed, Emacs (eglot) · documented tier: JetBrains (LSP4IJ)
- marketplace/packaging for all of the above

Which editors are supported, at what verification level, and when each was last
checked: [`docs/MATRIX.md`](docs/MATRIX.md). Every row there names its CI job or
says plainly that it has never been run.

Sprint plan: the `lsp` track (`lsNN`) in the wolf metarepo.
Dual-licensed MIT or Apache-2.0.

## The harness

`lspconf` drives a real `wolf lsp` child process over stdio. Half of it needs
no server and runs everywhere; the other half needs a binary at the pin and
**skips loudly** — exit `77`, with the reason — when there is none.

```
lspconf verify              transcripts and scripts: parseable, valid, canonical
lspconf profiles            capability profiles, and which clients are unread
lspconf doctor              the pin, the binary that won, the verdict
lspconf record <s.lsps>     drive a scripted session, write <s>.jsonl
lspconf rerecord [dir]      re-record every script beside its transcript
lspconf replay [path…]      drive recorded sessions, match per record
lspconf onetruth [sample…]  publishDiagnostics == conform-run, per sample (D34)
lspconf bench [--out F]     latency budgets, D5 JSONL (report-only)
lspconf fuzz [--seed N]     seeded partial-edit session, three oracles
```

A **transcript** (`transcripts/**/*.jsonl`) is a recorded JSON-RPC session,
normalized so it fails when behavior changes and not when incidental output
does. Beside each one is the **script** (`.lsps`) that produced it, so a
re-record is one command and its diff is reviewable. Never hand-edit a
transcript: `lspconf verify` refuses one that is not in canonical form, and one
with no script beside it.

## Running the server lane locally

CI cannot do this and is not supposed to: wolf-lang publishes no release
artifact yet, and this repo **never builds the compiler in CI**
(`vendor/README.md` explains why — a private submodule with no deploy keys, and
a multi-minute build per job to produce something the acquisition step is meant
to download). Locally it is three commands, once:

```sh
git submodule update --init upstream
cargo build --release -p wolf_driver --manifest-path upstream/Cargo.toml
export WOLF_BIN="$PWD/upstream/target/release/wolf"
```

Then `cargo run --bin lspconf -- doctor` should say `READY`, and everything
above works:

```sh
cargo run --bin lspconf -- --require-server replay     # the transcript library
cargo run --bin lspconf -- --require-server onetruth   # D34, falsifiable
cargo run --bin lspconf -- --require-server fuzz regions.lu --seed 1
cargo test                                             # the gated suites go live
```

Without `WOLF_BIN`, resolution falls back to `.wolf-bin/` (the artifact cache CI
will use) and then to `wolf` on `PATH` — and `doctor` reports which one won,
because "works on my machine" is usually a second `wolf` earlier in `PATH`.

## When the build and the editor disagree

`lspconf onetruth` is D34 made falsifiable: for every sample it runs
`wolf conform-run --error-format=json` and an LSP session over the same bytes
and asserts the diagnostics are the same — same codes, same spans (through the
negotiated position encoding), same messages, and reachable from *some* open
document of the module.

A mismatch is a **wolf-lang bug**, filed upstream with both records attached.
It is never normalized away here and never patched around: the editor layer
detects divergence, it does not hide it. `divergences.toml` is the ledger of
filed ones — an unfiled divergence fails, and so does a ledger entry whose bug
has been fixed.
