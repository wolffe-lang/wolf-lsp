# Clients

Editor-side integration, one subtree per client. **Deliberately outside the
cargo workspace**: a Rust workspace that swallows a node project is how the CI
lane for each stops being separable.

- [`fackr/`](fackr/README.md), `facsimile/` — tier 0, the daily drivers (ls02–ls03)
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
rest should follow.
