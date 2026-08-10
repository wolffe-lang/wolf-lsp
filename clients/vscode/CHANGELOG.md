# Changelog — Wolf for VS Code

User-visible changes only. A client changelog that recites internal refactors
trains people to stop reading it (ls07 §4.6), so nothing here describes a test,
a generator or a CI lane; those live in the campaign closeout.

`cargo xtask release-check` fails if the top entry does not name the version
`clients/vscode/compat.json` declares.

## 0.0.1 — UNRELEASED

**Never published.** There is no marketplace listing and no Open VSX namespace;
the only way to have this extension is to build the vsix from a checkout
(`README.md` §Installing). The date below is the date the version was declared,
not a release date.

- Wolf language support: diagnostics, hover, document symbols, formatting and
  code actions, all served by `wolf lsp` — the compiler itself. There is no
  separate server to install (D34).
- Syntax highlighting for `.lu`, `.wolfi` and `wolf.pkg` / `wolf.sum`, generated
  from the pinned grammar rather than hand-written.
- Settings: `wolf.serverPath` (empty means `wolf` on `PATH`) and
  `wolf.trace.server`.
- Commands: **Wolf: Restart Language Server**, **Wolf: Show Server Log**,
  **Wolf: Show Version**.
- A `wolf` problem matcher and a `wolf` task type, so `wolf build` output is
  clickable in the terminal.
- If `wolf` is missing, one non-modal notification with an install link — no
  retry loop, and no auto-download of a toolchain.
- If `wolf --version` falls outside the range in `compat.json`, one non-modal
  notification per session naming both versions. Editing is never blocked and
  the server always starts: an out-of-range server usually mostly works.
- Bundled: `LICENSE.md` (MIT **or** Apache-2.0) and `compat.json`, so the
  compatibility statement travels with the artifact.

Verified against `wolf 0.0.1 (pre-alpha)` at wolf-lang `70bdd35` — see
[`docs/COMPAT.md`](../../docs/COMPAT.md).
