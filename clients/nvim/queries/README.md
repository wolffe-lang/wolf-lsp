# `queries/wolf/` — deliberately empty, and why

These four files exist, are on the runtimepath, are loaded by Neovim the moment
a `wolf` parser appears, and contain **no patterns**. That is not an oversight
and it is not a placeholder anyone forgot to fill.

## The blocker

`wolffe-lang/tree-sitter-wolf` is scaffold-only. At the pin this plugin was
written against the repository contains `LICENSE` and
`README.md` — no `grammar.js`, no `src/`, no parser. Its stated scope is that
the grammar gets "filled opportunistically between compiler sprints", and ls04
is explicit that nothing in this sprint blocks on it.

A tree-sitter query is written against **node names**, and node names come from
the grammar. With no grammar there are no node names, so every pattern anyone
wrote here today would be a guess dressed as a derivation — the exact failure
`inventory.md` exists to prevent for the regex highlighter, transplanted into a
file format that fails louder. `vim.treesitter.query.get` raises on a query
naming a node the grammar does not have, so a speculative `highlights.scm`
would not degrade gracefully the day the grammar lands: it would break every
`.lu` buffer for everyone who installed the parser, and the breakage would look
like a grammar bug.

Writing zero patterns is the only option that is correct in both worlds.

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

When `tree-sitter-wolf` has a grammar:

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
