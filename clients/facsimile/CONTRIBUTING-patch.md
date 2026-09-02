# Patching facsimile from the wolf side

The wolf-lsp track does not own facsimile. It sends patches, and this file is
the agreement about how — so that a change made to serve wolf still reads as a
change that improves facsimile.

## Branch and commit shape

Branch off facsimile `trunk`. One PR per logical change; chunked commits
inside it; terse imperative subjects under ~250 characters; **no trailers of
any kind** — no `Co-Authored-By`, no generated-with. Never `git add -A`. This
mirrors the compiler, interpreter and editor tracks, and facsimile's own
history.

The decomposition is in [`patches/STATUS.md`](patches/STATUS.md).

## The gates, which are not negotiable

facsimile's CI is strict, and every one of these runs on push and PR:

- **A zero-compiler-warnings build.** `make` with `-Wall`. Not "few warnings" —
  none.
- **A 132-column source-line limit.** Fortran's free-form limit. Unicode
  characters count as their *bytes*, so a box-drawing `═` costs three columns.
  Long lines split with `&` continuation.
- **`make check-windows`** — the Windows `CreateProcessA` branches are
  syntax-checked on Linux, and a helper defined for only one platform fails the
  gate. D35: the patch must not break this.
- **`make check-render`** — nothing may draw past the frame buffer.
- **`make check-deps`** — added after this series, at `daa258f`. `SOURCES` is
  now a chain of real prerequisites (`Makefile:170-179`), not just recipe
  order, and this gate dry-runs a change to `syntax_highlighter_module.f90` and
  fails unless `renderer_module.f90` is in the rebuild plan. It exists because
  a field added to `syntax_highlighter_t` once recompiled the module but not
  `renderer_module.o`, flattening all highlighting.
- **`fpm test`** and the `test/integration_*.py` shards.

Run all of them before sending anything. The patch in `patches/` was verified
against every gate except `fpm test` (no `fpm` on the recording machine) and
the pexpect shards (they exercise the editor, not this change).

## Rules that are not negotiable either

**A fix is a fix for facsimile, not for wolf.** The formatting keybinding, the
installer count and the `/tmp` debug log are all wrong-for-every-server or
wrong-for-every-user bugs that wolf merely happened to trigger first. If a
proposed patch only makes sense when the server is `wolf`, it does not belong
upstream — that is a wolf-lsp problem wearing a Fortran costume.

**No wolf-specific branches in facsimile's code.** Not `if language == "wolf"`,
not a special case in the tokenizer, not a workaround for something `wolf lsp`
gets wrong. A server bug is a wolf-lang sprint. The one wolf-shaped thing in
the whole series is a row in a table of twenty other servers, plus a language
definition beside eleven others.

**Match the house style, do not import ours.** Four-space indent, `!`
comments, `block` constructs for locals, explicit `intent`. Comment the failure
rather than the code: the reason the `"""` delimiter is listed before `"` is
that `process_string` takes the first match and would otherwise make wolf's
block strings unreachable — *that* sentence belongs next to the array, and
`! string delimiters` does not.

**A new module means a Makefile edit.** `SOURCES` is explicit and
**dependency-ordered** (`.NOTPARALLEL` enforces sequential builds, so a module
must appear before anything that `use`s it), and new C files go in
`C_SOURCES`. This series added **no new files**, so `SOURCES` was untouched —
which was also the cheapest way to keep the patch reviewable.

Since `daa258f` that ordering is enforced rather than promised: each object is
given its predecessor as an actual prerequisite (`Makefile:170-179`), so a
module change rebuilds every consumer after it, and `make check-deps` fails if
the chain is broken. A patch that adds a module in the wrong place now fails a
gate instead of producing a stale binary.

## Things to leave alone

facsimile's LSP architecture is out of scope, on purpose (ls03 non-targets): no
rewrite of the JSON parser, no incremental sync, no cancellation, no async I/O,
no LSP config file, no tree-sitter, no semantic tokens, no bundled `wolf`
binary. Those are facsimile issues to file with evidence — and
`patches/STATUS.md` files them — not wolf-track work smuggled in behind a
language registration.

## The mirror

`patches/wolf-integration.diff` holds the series as a diff against the
facsimile commit it was written for, so this integration is reproducible from
this repo even if the PRs sit. When a PR lands, update its row in
`patches/STATUS.md` with the merge commit; when the diff no longer applies,
re-cut it against the new base rather than letting the mirror rot into fiction.
