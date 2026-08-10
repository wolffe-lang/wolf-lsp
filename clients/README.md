# Clients

Editor-side integration, one subtree per client. **Deliberately outside the
cargo workspace**: a Rust workspace that swallows a node project is how the CI
lane for each stops being separable.

- [`fackr/`](fackr/README.md), [`facsimile/`](facsimile/README.md) — tier 0, the daily drivers (ls02–ls03)
- [`nvim/`](nvim/README.md), [`vscode/`](vscode/README.md) — tier 1, the plugin tier (ls04–ls05)
- [`helix/`](helix/README.md), [`zed/`](zed/README.md), [`emacs/`](emacs/README.md) — tier 2, the config tier (ls06)
- [`jetbrains/`](jetbrains/README.md) — tier 3, documented only (ls06)

Every one of them configures a client to launch `wolf lsp`. None of them
implements a server capability, works around a missing one, or post-processes a
diagnostic (D22: diagnostics are the reviewed artifact; the editor layer must
not launder them).

For a client whose source we own (tier 0), the subtree carries no code at all —
the code goes upstream as a patch series and this directory keeps the mirror,
the provenance of any generated table, the recorded session's recipe, and an
honest account of what the client still cannot do. `fackr/` is the shape the
rest should follow; `facsimile/` adds a `CLIENT.md`, because a client whose
*limits* are the deliverable needs somewhere to state them that is not a
limitations section bolted onto a setup guide.

**Tier 1 breaks that rule, and has to.** `nvim/` and `vscode/` hold real code,
because the plugin is ours to write rather than someone else's to accept a patch
into — each is a plugin root that ships from this repository until distribution
is decided (ls07). Their READMEs keep the same shape and the same honesty
standard as the tier-0 ones; what they add is a test lane, because there is no
upstream CI to split the verification with.

The two tier-1 subtrees differ in one structural way worth knowing before
reading either. `nvim/`'s highlighting is a **hand-written** `syntax/wolf.vim`
with a check pointed at its keyword list; `vscode/`'s is a **generated**
`.tmLanguage.json` whose every byte is re-derived from the pinned grammar. That
is not a change of mind — it is a `.vim` file a human can read at a glance
versus two hundred lines of escaped regex, where a hand-edit in the middle is
invisible to any check that only inspects the terminal lists.

The four clients read so far are deliberately opposed, and that is most of
their value. fackr counts columns in code points and declares `["utf-32"]`;
facsimile counts them in UTF-16 code units and declares `["utf-16"]`; Neovim
declares all three with `utf-8` first and therefore negotiates `utf-8`. Between
them they reach every branch of wolf's encoding preference order with a client
someone actually types in, so a change to that order cannot pass the suite by
agreeing with itself.

VS Code lands on the same wire format as facsimile and is not redundant with it,
because it raises the *stakes* rather than adding a branch:
`vscode-languageclient` hardcodes `["utf-16"]` and **throws** on any other
answer, so where every other client here would mis-render a wrong encoding, this
one refuses to start. Same branch, different failure mode — and the harsher one
is the one that tells you immediately.

## The config tier, and why its three subtrees look nothing alike

`helix/`, `zed/` and `emacs/` are all "tier 2 — we ship a config" and the three
directories share not one file format, one verification mechanism, or one idea
of what a config even is. That is a property of the editors, not a failure to
standardise, and the shape of each subtree is the honest reading of its editor:

- **`helix/` is one TOML file and nothing else.** helix configures a language
  server declaratively, so there is no code to write and none is written. Its
  verification is `hx --health` — the editor's own parser, run in CI against the
  exact bytes the README tells you to append to your config.
- **`zed/` is a Rust crate**, because Zed's manifest can declare that a language
  server exists but cannot say how to *find* one: binary discovery is
  `language_server_command`, compiled to a WebAssembly component. So the "config
  tier" label is least true here, and `zed/README.md` opens by saying so.
- **`emacs/` is one elisp file that is simultaneously a snippet.** Emacs has no
  configuration format that is not a program. `wolf-mode.el` ships as a file so
  that CI can *run* it and is quoted verbatim in the README so a reader can paste
  it; a test fails if the two copies drift.

What they do share is the thing that matters: all three spawn `wolf lsp` and
nothing else (D34), none attaches a server to `.wolfi`, and none references
`tree-sitter-wolf` while that repository has no grammar in it. Those are
cross-editor invariants that no single editor can check, so
`cargo xtask config-check` checks them — including the formatter's two numbers
(`INDENT = 4`, `WIDTH = 100`), which every client states and which would
otherwise drift one file at a time.

`jetbrains/` is a page of prose with no artefact, and that is the whole
deliverable: the vendor LSP API is gated to the paid IDEs, and LSP4IJ needs no
plugin from us. Authoring one would buy nothing except a second thing to version.
