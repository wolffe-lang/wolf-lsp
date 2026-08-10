# Clients

Editor-side integration, one subtree per client. **Deliberately outside the
cargo workspace**: a Rust workspace that swallows a node project is how the CI
lane for each stops being separable.

- [`fackr/`](fackr/README.md), [`facsimile/`](facsimile/README.md) — tier 0, the daily drivers (ls02–ls03)
- `nvim/`, `vscode/` — tier 1, the plugin tier (ls04–ls05)
- `helix/`, `zed/`, `emacs/`, `jetbrains/` — tiers 2 and 3 (ls06)

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

The two tier-0 clients are deliberately opposed, and that is most of their
value. fackr counts columns in code points and declares `["utf-32"]`;
facsimile counts them in UTF-16 code units and declares `["utf-16"]`. Between
them they pin both ends of wolf's encoding preference order, so a change to
that order cannot pass the suite by agreeing with itself.
