# wolf-lsp

The wolf editor layer: clients, configs, conformance testing, and packaging
for the wolf language server.

**The engine is not here.** The language server is `wolf lsp` — the wolf
compiler itself, serving the Language Server Protocol from the same code
that compiles (one process, one truth; see wolf-lang's `wolf_query`
contract). This repo makes editors speak to it:

- the protocol **conformance harness** (recorded JSON-RPC session replay
  against `wolf lsp`, capability snapshots, latency budgets)
- first-class clients: **fackr**, **facsimile**, Neovim, VS Code
- config-native tier: Helix, Zed · documented tier: Emacs (eglot), JetBrains
- marketplace/packaging for all of the above

Sprint plan: the `lsp` track (`lsNN`) in the wolf metarepo.
Dual-licensed MIT or Apache-2.0.
