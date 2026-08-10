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
| [fackr](../clients/fackr/README.md) | 0 | registered, patched, recorded — the patch series is open upstream, not merged | `fackr/smoke` (client-recorded) | 2026-08-10, pin `67c977f` |
| [nvim](../clients/nvim/README.md) | 1 | plugin shipped from `clients/nvim/`; the config is upstreamable to nvim-lspconfig verbatim, and that PR is ls07's | `nvim/smoke` (client-recorded) | 2026-08-10, pin `70bdd35`, NVIM v0.12.4 |
| [vscode](../clients/vscode/README.md) | 1 | extension shipped from `clients/vscode/`; vsix is the install path, marketplace + Open VSX are ls07's | `vscode/smoke` (client-recorded) | 2026-08-10, pin `70bdd35`, VS Code 1.132.0 + vscode-languageclient 9.0.1 |
| _(ls03, ls06 fill the rest)_ | | | | |

Tiers (report 09 §client tier matrix):

- **Tier 0 — daily drivers** (fackr, facsimile). We own the client code;
  breakage is a release blocker.
- **Tier 1 — plugin tier** (Neovim, VS Code). We ship and version a plugin.
- **Tier 2 — config tier** (Helix, Zed). We ship a config snippet or a thin
  extension. Base LSP only.
- **Tier 3 — documented tier** (Emacs, JetBrains, Sublime, Kate). A working
  recipe, no shipped artefact, best-effort.
