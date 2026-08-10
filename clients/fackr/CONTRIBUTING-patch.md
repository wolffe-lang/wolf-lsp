# Patching fackr from the wolf side

The wolf-lsp track does not own fackr. It sends patches, and this file is the
agreement about how — so that a change made to serve wolf still reads as a
change that improves fackr.

## Branch and commit shape

Branch off fackr `trunk`. One PR per logical change; chunked commits inside it;
terse imperative subjects under ~250 characters; **no trailers of any kind** —
no `Co-Authored-By`, no generated-with. Never `git add -A`. This mirrors the
compiler, interpreter and editor tracks, and fackr's own history.

The decomposition for the wolf integration is in
[`patches/STATUS.md`](patches/STATUS.md): PR1 registration, PR2 syntax, PR3 the
installer panel, PR4 the client-correctness fixes (each commit independently
revertable), and an optional PR5 CI lane.

## Rules that are not negotiable

**A fix is a fix for fackr, not for wolf.** The framing rewrite, the stderr
drain and the encoding declaration are all wrong-for-every-server bugs that
wolf merely happened to trigger first. If a proposed patch only makes sense
when the server is `wolf`, it does not belong upstream — that is a wolf-lsp
problem wearing a fackr costume.

**No wolf-specific branches in fackr's code.** Not `if language == "wolf"`,
not a special case in the tokenizer, not a workaround for something `wolf lsp`
gets wrong. A server bug is a wolf-lang sprint. The one wolf-shaped thing in
the whole series is a row in a table of forty other servers.

**Every fix arrives with the test that would have caught it.** fackr had no
tests for its LSP module at all; each patch here adds the ones for the code it
touches. `src/lsp/process.rs`'s chunk-boundary test splits a 🐺 at every byte
offset of a real frame, because the bug it replaces was "sometimes, on some
input, silently".

**Match the house style, do not import ours.** fackr is edition 2021, is not
rustfmt-clean, and carries ~60 clippy warnings at `496c7e2`. So: format new
files with `rustfmt` and leave old ones alone (never run `cargo fmt` across the
tree — it would bury a 600-line patch in an 8,700-line reformat), and add no
new clippy warnings without fixing the pre-existing ones first, which is a
separate conversation with fackr's owner.

**Comment the failure, not the code.** The reason a byte-framing loop exists is
that a decode-per-read dropped whole messages when a character straddled an
8 KiB boundary. That sentence belongs next to the loop; `// read bytes` does
not.

## Things to leave alone

fackr's LSP architecture is out of scope, on purpose (ls02 non-targets): no
async runtime, no incremental sync, no `$/cancelRequest`, no pull diagnostics,
no LSP config file, no tree-sitter, no semantic tokens, no bundled `wolf`
binary. Those are fackr issues to file with evidence — and `patches/STATUS.md`
files them — not wolf-track work smuggled in behind a language registration.

## The mirror

`patches/` holds the series as a diff against the fackr commit it was written
for, so this integration is reproducible from this repo even if the PRs sit.
When a PR lands, update its row in `patches/STATUS.md` with the merge commit;
when the diff no longer applies, re-cut it against the new base rather than
letting the mirror rot into fiction.
