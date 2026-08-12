# Contributing to wolf-lsp

## The engine is not here

The language server is `wolf lsp`, the wolf compiler itself, serving LSP from
the same code that compiles (D34: one process, one truth). **This repo
contains no server code, and none is accepted here.**

No fallback. No "temporary" diagnostic parser. No formatting shim. No
client-side approximation of a capability that has not shipped. A second
implementation of anything server-side is the exact failure this track exists
to avoid: the moment the editor layer can answer a question itself, it stops
proving that the compiler can.

Concretely:

- **A capability an editor wants is a wolf-lang sprint, not a shim here.**
  Semantic tokens, inlay hints, rename, refactoring are all compiler-track
  post-v1. Their tests get written when the server ships them.
- **No cargo dependency on any `wolf_*` crate**, and no vendored wolf-lang
  source. This repo talks to a `wolf` process over stdio or it does not talk.
- **CI never builds the compiler.** It downloads the artifact wolf-lang
  publishes, keyed by `vendor/upstream/PIN`, or it skips loudly.

`cargo xtask independence` enforces all three, on every PR.

## Commits

Mirroring the compiler and interpreter tracks:

- **Commit in chunks.** One logical change per commit. A refactor and the
  feature it enables are two commits. A toolchain bump or a pin bump is always
  its own commit. Never `git add -A`.
- **Terse, imperative subjects**, under ~250 characters unless the change
  genuinely needs elaboration: `matcher: default ServerCapabilities to subset`,
  not `Added some logic so that capabilities can grow without...`.
- Never `git checkout` files that were just written but not yet committed.
  Stage or stash first. This has eaten work before.

## Tests are first class

- Every behaviour worth having is worth a test, and every bug fix arrives with
  the test that would have caught it. Tests land in the same commit as the code.
- CI is first class too. A red CI is a stop-the-line event, never a thing to
  merge around.
- `cargo xtask ci` is the local gate: fmt-check, `clippy -D warnings`, tests,
  `sync-pin`, `independence`, and the `lspconf doctor` report.

### The two halves

Everything in this repo is either **server-free** or **server-dependent**, and
the split is load-bearing:

- Server-free (transcript parsing, matchers, normalization, profile validation,
  pin integrity) must be green on a fresh clone **with no `wolf` on PATH**.
  That is why the matcher engine is a pure function of two JSON values.
- Server-dependent (record, replay, bench) must **skip loudly**: print
  `SKIP: <reason>` and exit `77`. Never exit 0 for work that did not happen.
  A lane that reports success for doing nothing is a lane nobody notices is
  dark. `--require-server` turns the skip into a failure, for jobs that are
  supposed to have a binary.

### The snapshot ritual (insta)

Snapshots live in `crates/*/tests/snapshots/`. When one legitimately changes:

```sh
INSTA_UPDATE=always cargo test        # rewrite the .snap files
git diff '**/snapshots/'              # READ the diff — this is the review
cargo test                            # verify-clean run against the new snaps
```

The middle step is the whole point. `INSTA_UPDATE=always` makes any diff
disappear, including the one that was a real regression. Reviewing the diff is
what turns that hazard into a workflow. Never commit a `.snap.new` or a
`.pending-snap`.

Snapshots hold the **normalized** view of a transcript and never the raw one,
so a diff means a behavior change and not a different run.

### Corpus samples are read-only

`vendor/upstream/samples/` is the only source of `.lu` in this repo, and its
bytes are canonical `wolf fmt` output at a STYLE_VERSION. **Never reformat a
sample**, never fork one into a local fixture, and never "fix" one that looks
wrong. That is a finding to report upstream. A sample whose bytes change is a
pin bump, in its own commit (`vendor/README.md`).

## Platform lessons

These were paid for once already, upstream. Do not re-derive them.

- **Line endings are protocol surface.** `.gitattributes` pins
  `* text=auto eol=lf` and it stays that way. LSP positions are derived from
  byte offsets, and a CRLF checkout silently shifts every column in every
  transcript. CI proves it held.
- **Capture, then grep. Never pipe a program's stdout into `grep -q`.**
  `grep -q` exits at the first match and closes the read end. The producer's
  exit-time flush then hits `EPIPE`, and Rust's default SIGPIPE-ignore turns
  that into a panic, nondeterministically, and it surfaced on macOS first.
  Write `out=$(cmd)` then `printf '%s' "$out" | grep -q …`.
- **Sort `read_dir` output** before it can influence any generated artifact or
  any message a human reads. Directory order is platform noise.
- **Never seed an RNG from a raw file path.** Normalize separators first. A
  Windows `\` explored an unvetted seed space compiler-side. ls01's edit fuzzer
  takes explicit seeds. More generally, no raw platform path reaches a
  transcript, a report key, or a hash: use `lsp_harness::slash_path`.
- **Pin CI toolchains explicitly.** `rust-toolchain.toml` names the toolchain.
  CI installs it with an explicit `rustup` step, then adds components. An
  implicit proxy-install races the first cargo invocation and picks up neither
  rustfmt nor clippy.
- **`Content-Length` counts bytes, not characters.** The most common framing
  bug in hand-written clients, which is most of the clients this track
  integrates with.

## Toolchain

Pinned in `rust-toolchain.toml` (rustup and CI) and `rust-version`
(everyone). Bump deliberately, in a dedicated commit, CI-green on all three
platforms.

## Model policy

Track agents run **Opus**. The work is integration against hand-rolled LSP
clients in three languages, where the failure modes are subtle (framing,
position encoding, unanswered server requests) and a wrong patch lands in
someone's daily driver. Agent guidance itself stays untracked: `AGENTS.md`
and `CLAUDE.md` are gitignored, as on the compiler and interpreter tracks.

## Before you push

```sh
cargo xtask ci
```

Green, on a clean tree. `wolf lsp` exists upstream as of s52, so the
server-dependent lanes light up as soon as a binary at the pinned version
resolves. Without one, `lspconf doctor` reports `SERVER UNAVAILABLE`, which is
a skip and not a failure, and `xtask ci` treats it as one.

## Where the plan lives

The sprint plan is the `lsp` track (`lsNN`) in the wolf metarepo. Sprint files
are the implementation contract, and the decision log is the design authority.
