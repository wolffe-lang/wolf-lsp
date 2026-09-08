# wolf-lsp

<img src="assets/wolf-logo.svg" alt="the wolf mark" width="120" align="right"/>

The wolf editor layer: clients, configs, conformance testing, and
packaging for the wolf language server.

The language server itself lives in the compiler. `wolf lsp` is the
wolf compiler serving the Language Server Protocol from the same code
that compiles, so the editor and the build see the same diagnostics
(wolf-lang's `wolf_query` contract). This repo makes editors talk to
it:

- a protocol conformance harness: recorded JSON-RPC session replay
  against `wolf lsp`, capability snapshots, latency budgets
- maintained clients for fackr, facsimile, Neovim and VS Code
- configuration for Helix, Zed and Emacs (eglot), and documentation for
  JetBrains (LSP4IJ)
- marketplace and packaging for all of the above

[`docs/MATRIX.md`](docs/MATRIX.md) lists which editors are supported,
at what verification level, and when each was last checked. Every row
names its CI job or states that it has never been run.

None of the clients have been published to a marketplace, registry or
mirror yet, and there is no tag. The pipelines exist and run offline;
each stops at a step that needs a person.

| | |
|---|---|
| [`docs/COMPAT.md`](docs/COMPAT.md) | which `wolf` each client works with, and the check that keeps the claim current |
| [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md) | every channel, per ecosystem, and what each one is waiting on |
| [`docs/RELEASE.md`](docs/RELEASE.md) | the release checklist, steps 0 through 9. `cargo xtask release-check` runs it |
| [`docs/UPSTREAM.md`](docs/UPSTREAM.md) | the status of every patch sent upstream |

Licensed under [GPL-3.0-or-later](LICENSE).

## The harness

`lspconf` drives a real `wolf lsp` child process over stdio. Half of it
needs no server and runs anywhere. The other half needs a binary at the
pin and, without one, exits `77` with the reason.

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

A transcript (`transcripts/**/*.jsonl`) is a recorded JSON-RPC session,
normalized so that it changes when behaviour changes and stays the same
when incidental output changes. Beside each one is the script (`.lsps`)
that produced it, so re-recording is one command and its diff is
reviewable. Transcripts are generated, and `lspconf verify` rejects one
that is not in canonical form or has no script beside it.

## Running the server lane locally

CI does not build the compiler. wolf-lang is a binary dependency here,
and building the whole compiler in every job would cost minutes to
produce what the acquisition step downloads (`vendor/README.md` has the
detail). Locally it is three commands, once:

```sh
git submodule update --init upstream
(cd upstream && cargo xtask dist)
export WOLF_BIN="$PWD/upstream/target/release/wolf"
```

The build stamp matters (D57), and it has to come from the builder. An
unstamped build prints `+dev.unknown`, the same string every trunk
build of that crate version prints, and `doctor` rejects it: a version
string that cannot name its commit is the stale-binary problem the pin
exists to prevent.

Two things about the upstream stamp explain why this is `xtask dist`
and no longer a hand-rolled `cargo build` with `WOLF_COMMIT` set:

- A release stamp needs `WOLF_RELEASE=v{version}` as well as
  `WOLF_COMMIT`, and upstream grants it only when that exact tag points
  at HEAD. When the pin is a release tag, a `WOLF_COMMIT`-only build
  prints `<version>+dev.<sha>`, which does not match the bare version
  the PIN records.
- The old recipe abbreviated the sha itself, as `--short=7`. Upstream's
  stamp uses `--short` with git's automatic width, which is eight for
  wolf-lang today, so the two differed by one character and `doctor`
  rejected the difference. Running upstream's builder gives upstream's
  answer.

`cargo xtask dist` also stages a tarball under `upstream/target/dist/`.
The binary at `upstream/target/release/wolf` is the same build.

Then `cargo run --bin lspconf -- doctor` should say `READY`, and the
gated commands work:

```sh
cargo run --bin lspconf -- --require-server replay     # the transcript library
cargo run --bin lspconf -- --require-server onetruth   # D34
cargo run --bin lspconf -- --require-server fuzz regions.lu --seed 1
cargo test                                             # the gated suites go live
```

Without `WOLF_BIN`, resolution falls back to `.wolf-bin/`, the artifact
cache CI uses, and then to `wolf` on `PATH`. `doctor` reports which one
won, because "works on my machine" is usually a second `wolf` earlier in
`PATH`.

## When the build and the editor disagree

`lspconf onetruth` tests D34 directly. For every sample it runs
`wolf conform-run --error-format=json` and an LSP session over the same
bytes, then checks that the diagnostics agree: same codes, same spans
(through the negotiated position encoding), same messages, and
reachable from some open document of the module.

A mismatch is a wolf-lang bug and is filed upstream with both records
attached. This repo does not normalize or patch around it, because
detecting divergence is what the harness is for. `divergences.toml` is
the ledger of filed mismatches. An unfiled divergence fails the gate,
and so does a ledger entry whose bug has since been fixed.
