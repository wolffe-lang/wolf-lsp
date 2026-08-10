# fackr patch series — status

The mirror exists so this integration is reproducible from *this* repo even if
the upstream PRs sit unreviewed. `wolf-integration.diff` is the whole series
against `fackr@496c7e2` (`trunk`); the table below is the decomposition an
integrator commits it in, one PR per group, chunked commits inside each.

Apply with `git apply` from a fackr checkout, or read the branch directly:
`wolf-lsp-integration` in the author's fackr worktree.

## The series

| PR | files | change | status |
|----|-------|--------|--------|
| **PR1** registration | `src/lsp/types.rs` (`detect_language`), `src/lsp/manager.rs` (`register_default_configs`) | `.lu`/`.wolfi` → languageId `wolf`; `ServerConfig::new("wolf", "wolf", ["wolf", "lsp"])` | in worktree, uncommitted |
| **PR2** syntax | `src/syntax/languages.rs` (5 edits), `src/syntax/highlight.rs` (tests) | `Language::Wolf`, `wolf_def()` generated from the pinned grammar, two `mod tests` cases | in worktree, uncommitted |
| **PR3** installer panel | `src/lsp/server_manager.rs` | a `KnownServer` row whose `install_cmd` starts with `#`, so it routes to the manual-info dialog instead of `sh -c` | in worktree, uncommitted |
| **PR4a** framing | `src/lsp/process.rs` | `FrameBuffer`: accumulate bytes, frame on bytes, decode whole messages only. 7 unit tests, one of which splits a 🐺 at **every** byte offset | in worktree, uncommitted |
| **PR4b** stderr | `src/lsp/process.rs` | drain thread with a 64-line tail, so a chatty server cannot deadlock at ~64 KiB and a dying one can still be asked why | in worktree, uncommitted |
| **PR4c** position encoding | `src/lsp/protocol.rs`, `src/lsp/types.rs`, `src/lsp/manager.rs`, `src/lsp/client.rs` | declare `general.positionEncodings: ["utf-32"]`, read the reply into `ManagedServer::position_encoding`, warn when it is not utf-32 | in worktree, uncommitted |
| **PR4d** server messages | `src/lsp/message.rs`, `src/lsp/manager.rs`, `src/lsp/client.rs` | `window/logMessage` / `window/showMessage` reach a bounded `LspClient::server_log()` instead of `let _ = params` | in worktree, uncommitted |
| **PR-test** live smoke | `src/lsp/smoke_wolf.rs` (new), `src/lsp/mod.rs` | two `#[test]`s driving fackr's own client against a real `wolf lsp`, skipping loudly with no binary | in worktree, uncommitted |
| **PR5** CI lane | — | **offered, not written.** See README §"The CI lane fackr does not have" | not started |

`Cargo.lock` also carries a one-line refresh (`fackr 1.1.2` → `1.2.1`, catching
the lockfile up to `Cargo.toml`). It is unrelated to this work and belongs in
its own commit, or dropped.

## Nothing here is committed

Per the ls02 working agreement, every change sits uncommitted in a dedicated
worktree branch for review. The diff in this directory is the reviewable
artifact; the branch is the working copy.

## What was deliberately not patched

Filed as fackr issues with evidence, not fixed here — they are architecture,
and ls02's non-targets are explicit that its scope stops short of it:

- `send_request` busy-waits up to 5 s **on the UI thread** for the server to
  become `Ready`; a slow cold server freezes the editor.
- The initialize response is detected without checking the request id, and
  string-typed JSON-RPC ids are dropped entirely.
- `rootUri` is built by string concatenation, with no percent-encoding.
- Diagnostics render as a single gutter dot at `range.start.line`; the
  `message` text is never displayed, and `relatedInformation`/`tags` are parsed
  and discarded. D22's work goes to die here, and it is the highest-value
  remaining fackr patch.
- `document_saved` exists with zero call sites: fackr advertises
  `synchronization.didSave` and never sends `didSave`.
- `workspace/applyEdit` is refused with `-32601` despite `applyEdit: true`.
- The server is SIGKILLed microseconds after `exit`, which is why the recorded
  transcript ends at the `shutdown` response (README §"Known limitations").
