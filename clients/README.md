# Clients

Editor-side integration, one subtree per client. **Deliberately outside the
cargo workspace**: a Rust workspace that swallows a node project is how the CI
lane for each stops being separable.

- [`fackr/`](fackr/README.md), [`facsimile/`](facsimile/README.md) — tier 0, the daily drivers (ls02–ls03)
- [`nvim/`](nvim/README.md), `vscode/` — tier 1, the plugin tier (ls04–ls05)
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

**Tier 1 breaks that rule, and has to.** `nvim/` holds real code, because the
plugin is ours to write rather than someone else's to accept a patch into — it
is a plugin root (`lua/`, `lsp/`, `ftplugin/`, `syntax/`, `doc/`) that ships
from this repository until distribution is decided (ls07). Its README keeps the
same shape and the same honesty standard as the tier-0 ones; what it adds is a
test lane, because there is no upstream CI to split the verification with.

The three clients read so far are deliberately opposed, and that is most of
their value. fackr counts columns in code points and declares `["utf-32"]`;
facsimile counts them in UTF-16 code units and declares `["utf-16"]`; Neovim
declares all three with `utf-8` first and therefore negotiates `utf-8`. Between
them they reach every branch of wolf's encoding preference order with a client
someone actually types in, so a change to that order cannot pass the suite by
agreeing with itself.
