# `queries/wolf/` — deliberately empty, and why

These four files exist, are on the runtimepath, are loaded by Neovim the moment
a `wolf` parser appears, and contain **no patterns**. That is not an oversight
and it is not a placeholder anyone forgot to fill.

## The blocker, then and now

When this plugin was written (ls04), `wolffe-lang/tree-sitter-wolf` was
scaffold-only: `LICENSE` and `README.md`, no `grammar.js`, no `src/`, no
parser. A tree-sitter query is written against **node names**, and node names
come from the grammar, so every pattern written then would have been a guess
dressed as a derivation — the exact failure `inventory.md` exists to prevent
for the regex highlighter, transplanted into a file format that fails louder.
`vim.treesitter.query.get` raises on a query naming a node the grammar does not
have, so a speculative `highlights.scm` would not have degraded gracefully the
day the grammar landed: it would have broken every `.lu` buffer for everyone
who installed the parser, and the breakage would have looked like a grammar
bug. Writing zero patterns was the only option correct in both worlds.

The grammar landed at le02 and has a corpus gate of its own; helix and zed pin
it at `1834e73` (tl16), and tree-sitter-wolf ships
`queries/{highlights,locals,injections}.scm` of its own. **These four files
are still empty because nobody has written them**, not because there is
nothing to write them against — this README said "scaffold-only" until tl16.
Filling them is a lane of its own (below); until it runs, `:checkhealth wolf`
reports `0 pattern(s)` for each, which is the visible state this layout was
designed to show.

## Why the files exist at all, then

Because the *wiring* is the deliverable, and it is real:

- `lua/wolf/treesitter.lua` registers the `wolf` language for the `wolf`
  filetype, so an installed parser is used with no further configuration;
- these files being here means highlight-group churn is a change to **this**
  repo and never a `tree-sitter-wolf` release — the queries were deliberately
  not put in the grammar repo;
- `:checkhealth wolf` reports each file's compiled pattern count, so "0
  patterns" is a visible state rather than a silent one, and a query that
  stops compiling against a future grammar is reported as an error against
  this repo with the parser's own message.

The fallback is not a downgrade today: `syntax/wolf.vim` is derived from the
same pinned grammar the parser will be generated from, and it is the story
users actually get. `:checkhealth` says so in those words, as information
rather than a warning.

## Filling them

The owed lane, against the pinned grammar (tree-sitter-wolf's own
`queries/highlights.scm` is the starting point; Neovim's capture names differ
from helix's and zed's, so it is a port, not a copy):

1. `highlights.scm` — map node names onto the capture set `syntax/wolf.vim`
   already establishes (`@keyword`, `@type`, `@string`, `@comment.documentation`
   for `///`/`//!`, `@function`), so the two highlighters agree and switching
   between them is not a visual jump.
2. `injections.scm` — the two injections wolf actually has: `re"…"` bodies as
   `regex`, and `{…}` f-string interpolations back into `wolf`. Both are
   listed as gaps the regex highlighter cannot close.
3. `folds.scm` — blocks and items. `ftplugin/wolf.lua` already switches
   `foldmethod` to `expr` when a parser is present and leaves folding alone
   when it is not.
4. `indents.scm` — 4 spaces, per `wolf_fmt::doc::INDENT`.

Nothing else in the plugin has to change: the same plugin version starts using
the grammar the moment a parser is installed.
