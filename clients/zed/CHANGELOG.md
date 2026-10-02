# Changelog — zed_wolf

User-visible changes only (ls07 §4.6).

## 0.0.1 — UNRELEASED

**Read the caveat first: no human or CI job has ever run Zed against this
extension.** The wasm component builds for `wasm32-wasip2` in CI and the config
is statically checked, and that is the entire claim (`docs/MATRIX.md`). It is
not registered in Zed's extension registry and installing it means
`zed: install dev extension`, which is a GUI action.

- A Zed extension supplying `wolf` as a language server: `language_server_command`
  resolves the binary, honouring an `"lsp": { "wolf": { "binary": { "path": … } } }`
  override in `settings.json`.
- `languages/wolf/config.toml` and `languages/wolfi/config.toml`: comment form,
  brackets, indent.
- The server is attached to `Wolf` only, never to `Wolfi` (D32).
- `[grammars.wolf]` is **live**, pinning `wolffe-lang/tree-sitter-wolf` by
  commit (`1834e73` at tl16, the same rev helix names), and
  `languages/wolf/highlights.scm` ships beside it — since tl16 byte-identical
  to tree-sitter-wolf's own `queries/highlights.scm` at that rev. It shipped
  commented out at first, and that mattered more in Zed than anywhere else:
  Zed builds every grammar named in the manifest *at install time*, so a block
  pointing at the then-empty `tree-sitter-wolf` repository would have failed
  the install and taken the language server down with it. Live since le02,
  when the grammar landed.
- No runtime version check. The extension's only entry point is
  `language_server_command`, which Zed calls before there is anywhere to raise a
  notification; [`docs/COMPAT.md`](../../docs/COMPAT.md) carries the statement.

Declared against `wolf 0.0.1 (pre-alpha)` at wolf-lang `70bdd35`. The range is
**inherited from the pin the other clients were verified at, not earned by a Zed
session** — `profiles/zed.json` and `transcripts/zed/smoke.jsonl` are still owed.
