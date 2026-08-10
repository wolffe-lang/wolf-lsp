# fixtures

**One file lives here, and it is here under protest.**

The standing rule (ls00 §5) is that `.lu` fixtures are *requested upstream,
never forked locally*: a file authored in this repo is not canonical `wolf fmt`
output at a STYLE_VERSION, does not participate in the compiler's own corpus
runs, and drifts silently from the language it claims to be written in.
`vendor/upstream/samples/` is the only source of `.lu` for every test in this
repo, and `cargo xtask vendor-check` enforces that a sample there is listed in
`samples.toml` with the editor reason it was picked.

That rule met its exception at ls01 §5. Position encoding is the protocol wart
that corrupts buffers *silently* — a wrong `character` does not throw, it puts
the squiggle two columns left and the user learns not to trust squiggles. The
suite that catches it needs astral-plane text (one code point, two UTF-16 code
units, four bytes), combining marks, and a ZWJ sequence. No file in the
wolf-lang corpus at the pin contains any code point above U+FFFF; a scan of all
149 of them at the s52 bump still finds none. Shipping the editor layer with
its hardest encoding cases unwritten is a worse outcome than shipping one local
file, so the file exists.

## The policy that keeps the rule intact everywhere else

1. **A fixture exists only where `samples.toml` records a `[gap.*]` entry**
   naming it as that gap's `local_stopgap`. No gap, no fixture.
2. **Each fixture names its gap in its own header**, so the file explains
   itself to whoever finds it in three sprints.
3. **The upstream request is not withdrawn by the workaround.** The gap entry
   stays open; the fixture is a stopgap, not a decision.
4. **A fixture is deleted, not migrated,** on the pin that carries the real
   corpus sample — in that same commit. A local fixture that outlives its gap
   is a fork, and a fork is what this whole arrangement exists to avoid.
5. `cargo xtask fixtures-check` enforces 1–3 mechanically: every `.lu` here is
   claimed by a `local_stopgap`, every `local_stopgap` exists, and no second
   file has quietly joined the first.

## The one file

- `astral.lu` — `[gap.astral_plane]`. BMP multi-byte (`é`, `中`), astral plane
  (`🐺`, alone and paired), a combining mark, a ZWJ family sequence, a literal
  tab, a very long line, and a deliberate `E0003` whose span sits **after**
  astral text on the same line — so the reported `character` is a different
  number in each of the three encodings and at most one of them can be right.
  A conversion bug that a pure-ASCII fixture cannot see fails here by
  construction.

CRLF is not represented by a file here and never will be: `.gitattributes`
pins `eol=lf` for the whole repo, and a CRLF fixture would be normalized on
checkout into a file that no longer tests what it claims. CRLF reaches the
server the way it reaches a real one — as `didChange` *content*, from a script
(`transcripts/encoding/crlf-content.lsps`).
