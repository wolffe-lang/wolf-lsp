# vendor/upstream

A tracked **data** snapshot of wolf-lang at the commit recorded in
`upstream/PIN` — byte-identical to the `upstream/` submodule at that pin.

It holds exactly three things:

- `PIN` — the sha, the `wolf --version` string that sha produces, and whether
  `wolf lsp` exists at it.
- `spec/grammar.ebnf` — ls05's TextMate/tree-sitter drift check reads it.
- `samples/` + `samples.toml` — the only `.lu` files any test in this repo may
  touch, and the editor reason each was picked.

## Why this exists, and why CI never builds the compiler

The interpreter track paid for this lesson: the upstream repo is **private**
and org policy disables deploy keys, so CI cannot clone the submodule at all
(`wolf-interp/vendor/README.md`). A tracked snapshot keeps CI hermetic.

This track adds a second reason. wolf-lang is a **binary** dependency here, not
a source one: `wolf lsp` is a program this repo drives over stdio, so building
the whole compiler in every job would be a multi-minute tax on a repo whose own
code compiles in seconds — to produce something the acquisition step is
supposed to *download*. CI acquires the tier-1 artifact wolf-lang's
`xtask dist` publishes, keyed by `PIN`, or it acquires nothing and says so.

**There is no third option.** No vendored compiler source, no cargo dependency
on any `wolf_*` crate, no `cargo build` in `upstream/`. `cargo xtask
independence` enforces all three.

## Rules

- **Never edit files here by hand.** Re-vendor on pin bumps (below).
- **Never reformat a sample.** Corpus bytes are canonical `wolf fmt` output at
  a STYLE_VERSION; reformatting forks the pin. A sample whose bytes change is a
  pin bump, in its own commit.
- If a sample looks wrong, that is a finding to report upstream, not a patch to
  apply here.
- Retire this directory if wolf-lang becomes readable to CI (public at v1, or
  deploy keys enabled).

## The submodule

`upstream/` is the wolf-lang repo as a git submodule pinned to an exact commit.
It is the pin's source of truth and a **local convenience** — a developer may
`cargo build --release` there to get a `wolf` binary — but it is never the CI
acquisition path. A fresh clone does not need it:

```sh
git submodule update --init upstream    # optional; local only
```

Everything in this repo falls back to `vendor/upstream/` when the submodule is
absent (`lsp_harness::upstream_root`), which is always the case in CI.

## The pin-bump ritual

One commit, touching the `upstream` gitlink, `vendor/upstream/**`, and nothing
else. Subject: `upstream: pin bumped to <short> (<reason>); re-vendored`.

```sh
# 1. Move the submodule to the new commit.
git -C upstream fetch
git -C upstream checkout <sha>

# 2. Re-vendor the data. Sample paths are corpus-relative, so this is a
#    mechanical copy — read samples.toml for the list, never improvise one.
mkdir -p vendor/upstream/spec
cp upstream/spec/grammar.ebnf vendor/upstream/spec/grammar.ebnf
#    …and each `path` from samples.toml, from upstream/corpus/<path>
#      to vendor/upstream/samples/<path>.

# 3. Record the new pin. `version` must be what the NEW binary prints.
git -C upstream rev-parse HEAD          # -> commit
cargo build --release --manifest-path upstream/Cargo.toml
./upstream/target/release/wolf --version   # -> version
#    …and flip `serves_lsp = true` the pin that first ships `wolf lsp`
#      (wolf-lang s52). That flip is what lights up every dark CI lane.

# 4. Verify and commit.
cargo xtask sync-pin
git add upstream vendor/upstream
```

`cargo xtask sync-pin` compares the snapshot to the submodule byte-for-byte in
both directions (a stale extra file in `vendor/` is drift too) and validates
`PIN` and `samples.toml` even when the submodule is absent. It runs in CI, where
only the second half can execute.
