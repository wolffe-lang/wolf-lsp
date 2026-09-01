# wolf-lsp

<img src="assets/wolf-logo.svg" alt="the wolf mark" width="120" align="right"/>

The wolf editor layer: clients, configs, conformance testing, and packaging
for the wolf language server.

**The engine is not here.** The language server is `wolf lsp`, which is the
wolf compiler itself serving the Language Server Protocol from the same code
that compiles (one process, one truth; see wolf-lang's `wolf_query`
contract). This repo makes editors speak to it:

- the protocol **conformance harness**: recorded JSON-RPC session replay
  against `wolf lsp`, capability snapshots, latency budgets
- maintained clients: **fackr**, **facsimile**, Neovim, VS Code
- config tier: Helix, Zed, Emacs (eglot) · documented tier: JetBrains (LSP4IJ)
- marketplace and packaging for all of the above

Which editors are supported, at what verification level, and when each was last
checked: [`docs/MATRIX.md`](docs/MATRIX.md). Every row there names its CI job or
says plainly that it has never been run.

**Nothing here has been published anywhere.** No marketplace listing, no Open
VSX namespace, no `wolf.nvim` mirror, no registry entry, no tag. wolf-lang has
since published v0.1.0 with an x86-64 linux tarball, and the other tier-1
platforms have no artifact yet. The pipelines exist and are exercised offline.
They stop at gates that need a person:

| | |
|---|---|
| [`docs/COMPAT.md`](docs/COMPAT.md) | which `wolf` each client works with, and the gate that keeps the claim earned |
| [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md) | every channel, per ecosystem, and the human act each one waits on |
| [`docs/RELEASE.md`](docs/RELEASE.md) | the checklist, steps 0 through 9. Run it with `cargo xtask release-check` |
| [`docs/UPSTREAM.md`](docs/UPSTREAM.md) | every patch's status upstream, in five words, none of them "soon" |

Sprint plan: the `lsp` track (`lsNN`) in the wolf metarepo.
Licensed under [GPL-3.0-or-later](LICENSE).

## The harness

`lspconf` drives a real `wolf lsp` child process over stdio. Half of it needs
no server and runs everywhere. The other half needs a binary at the pin, and
**skips loudly** when there is none: exit `77`, with the reason.

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

CI cannot do this and is not supposed to. This repo **never builds the
compiler in CI**: wolf-lang is a binary dependency here, so building the whole
compiler in every job would be a multi-minute tax to produce something the
acquisition step is meant to download (`vendor/README.md` has the long
version). Locally it is three commands, once:

```sh
git submodule update --init upstream
(cd upstream && cargo xtask dist)
export WOLF_BIN="$PWD/upstream/target/release/wolf"
```

The build stamp is load-bearing since D57, and **the builder must compute it**,
not the caller. An unstamped build prints `+dev.unknown` — the same string
every trunk build of that crate version prints — and `doctor` refuses it,
because a version string that cannot name its commit is exactly the
stale-binary hole the pin exists to close.

Two things changed at le04, which is why this is `xtask dist` and no longer a
hand-rolled `cargo build` with `WOLF_COMMIT` in front of it:

- A **release** stamp needs `WOLF_RELEASE=v{version}` as well as
  `WOLF_COMMIT`, and upstream grants it only when that exact tag points at
  HEAD. Our pin is now a release tag (`v0.2.1`), so a `WOLF_COMMIT`-only build
  prints `0.2.1+dev.<sha>` and never the bare version the PIN records.
- The old recipe abbreviated the sha itself, as `--short=7`. Upstream's own
  stamp uses `--short` with git's **auto** width, which is eight for wolf-lang
  today, so the two disagree by one character and `doctor` refuses the
  difference. Running upstream's builder means the stamp is upstream's
  answer rather than our restatement of it.

`cargo xtask dist` also stages a tarball under `upstream/target/dist/`; ignore
it, or use it — the binary at `upstream/target/release/wolf` is the same build.

Then `cargo run --bin lspconf -- doctor` should say `READY`, and everything
above works:

```sh
cargo run --bin lspconf -- --require-server replay     # the transcript library
cargo run --bin lspconf -- --require-server onetruth   # D34, falsifiable
cargo run --bin lspconf -- --require-server fuzz regions.lu --seed 1
cargo test                                             # the gated suites go live
```

Without `WOLF_BIN`, resolution falls back to `.wolf-bin/`, the artifact cache
CI uses, and then to `wolf` on `PATH`. `doctor` reports which one won, because
"works on my machine" is usually a second `wolf` earlier in `PATH`.

## When the build and the editor disagree

`lspconf onetruth` is D34 made falsifiable. For every sample it runs
`wolf conform-run --error-format=json` and an LSP session over the same bytes,
then asserts the diagnostics agree: same codes, same spans (through the
negotiated position encoding), same messages, and reachable from *some* open
document of the module.

A mismatch is a **wolf-lang bug**, filed upstream with both records attached.
It is never normalized away here and never patched around. The editor layer
detects divergence; hiding it would defeat the harness. `divergences.toml` is
the ledger of the filed ones. An unfiled divergence fails the gate, and so
does a ledger entry whose bug has been fixed.
