# Editor support matrix

**Filled by ls06.** Per editor: what "supported" means at that tier, which
capabilities are exercised by a transcript, and when it was last verified
against which wolf pin.

Nothing goes in this table that is not backed by a green transcript in
`transcripts/`. Editor support here is *tested*, not asserted — a row whose
"last verified" is a date and not a CI run is a claim, and this file does not
carry claims.

| editor | tier | status | transcripts | last verified |
|---|---|---|---|---|
| _(ls02–ls06 fill this)_ | | | | |

Tiers (report 09 §client tier matrix):

- **Tier 0 — daily drivers** (fackr, facsimile). We own the client code;
  breakage is a release blocker.
- **Tier 1 — plugin tier** (Neovim, VS Code). We ship and version a plugin.
- **Tier 2 — config tier** (Helix, Zed). We ship a config snippet or a thin
  extension. Base LSP only.
- **Tier 3 — documented tier** (Emacs, JetBrains, Sublime, Kate). A working
  recipe, no shipped artefact, best-effort.
