# facsimile patch series — status

The mirror exists so this integration is reproducible from *this* repo even if
the upstream PRs sit unreviewed. `wolf-integration.diff` is the whole series
against `facsimile@1242ffa` (`trunk`); the table below is the decomposition an
integrator commits it in, one PR per group, chunked commits inside each.

Apply with `git apply` from a facsimile checkout, or read the branch directly:
`wolf-lsp-integration` in the author's facsimile worktree.

## The series

| PR | files | change | status |
|----|-------|--------|--------|
| **PR1** registration | `src/lsp/lsp_server_manager_module.f90` (`get_language_for_file`, `load_default_configs`), `src/lsp/lsp_client_module.f90` (`detect_language_from_extension`) | `.lu`/`.wolfi` → languageId `wolf`; `add_config(manager, "wolf", "wolf", "wolf lsp", "*.lu,*.wolfi", caps)` with only the four flags wolf serves | in worktree, uncommitted |
| **PR2** installer entry | `src/lsp/server_detection_module.f90` | count 20 → 21, the missing `i = i + 1` after the fortls block, and a `wolf` row whose `install_cmd` starts with `#` so it routes to the manual-info dialog instead of `sh -c` | in worktree, uncommitted |
| **PR3** syntax + comments | `src/syntax/syntax_highlighter_module.f90` (`detect_language`, new `load_wolf_syntax`), `src/syntax/comment_syntax_module.f90` (`get_comment_syntax`) | 50 keywords and 4 types generated from the pinned grammar; `//` line comments and no block form; `"""` listed before `"` | in worktree, uncommitted |
| **PR4a** position encoding | `src/lsp/lsp_protocol_module.f90` | declare `general.positionEncodings: ["utf-16"]` — correct-by-declaration rather than correct-by-default | in worktree, uncommitted |
| **PR4b** formatting keybinding | `src/commands/command_handler_module.f90` | `case('alt-shift-f', 'shift-alt-f')` — the handler matched a string the input layer never produces, so document formatting could not be invoked at all | in worktree, uncommitted |
| **PR4c** hot-path hygiene | `src/lsp/lsp_server_manager_module.f90` | drop the unconditional `/tmp/fac_didchange_debug.log` append from `notify_file_changed` | in worktree, uncommitted |
| **PR5** server-request replies | — | **offered, not written.** See "Deliberately not patched" | not started |
| **PR-test** live smoke | — | **offered, not written.** See `../README.md` §"The CI lane facsimile does not get" | not started |

**No new source files**, so the Makefile's dependency-ordered `SOURCES` list is
untouched — deliberately, because it keeps the patch reviewable and cannot
perturb the build order.

## Gate results with the series applied

Run in the worktree at `1242ffa` + the diff:

| gate | result |
|------|--------|
| `make` (zero warnings, `-Wall`) | **pass** — no warnings emitted |
| 132-column limit | **pass** — no `.f90` line exceeds it |
| `gfortran -Werror=line-truncation` sweep | **pass** — no truncation |
| `make check-windows` | **pass** — all four C files ok, no one-sided `lsp_` helpers |
| `make check-render` | **pass** |
| `make test-lsp` | **pass** — json, init, write-deadlock, stderr-drain |
| `fpm test` | **not run** — no `fpm` on the recording machine |
| `test/integration_*.py` | **not run** — they exercise editor behaviour this series does not touch |

## Nothing here is committed

Per the working agreement, every change sits uncommitted in a dedicated
worktree branch for review. The diff in this directory is the reviewable
artifact; the branch is the working copy.

## Reproducing the recorded session

The transcript was made by driving the patched `fac` through a pty with
`pexpect` + `pyte`, with the capture proxy first on `PATH` (recipe in
`../README.md`). The key sequence, after the editor settles:

| step | bytes | why |
|------|-------|-----|
| break the file | `x` | insert at 1:1, outside any declaration → E0203 |
| wait | — | > 0.5 s, the `document_sync_module` debounce |
| fix it | `\x7f` | backspace; the clean publish must return |
| move to `who` | `\x1b[B`×9, `\x1b[C`×8 | line 10, column 9 — hovering the header comment is a legitimate `null` and proves nothing |
| hover | `\x08` | ctrl-h |
| symbols | `\x1bOS` | F4 as SS3 |
| format | `\x1b[102;4u` | kitty CSI-u: codepoint 102 = `f`, mods 4 = alt+shift |
| quit | `\x11` | ctrl-q; no `shutdown`/`exit` is ever sent |

## Deliberately not patched

Filed with evidence, not fixed here — they are architecture or they belong to
another language's table, and ls03's non-targets stop short of both.

- **`handle_request` still answers nothing.** Replying `[]` to
  `workspace/configuration` and acking `client/registerCapability` would be
  worth far more than this whole series — it is the difference between "works"
  and "wedges forever" for *any* server, and several mainstream servers do send
  those. It is left out because it is a behaviour change to the message loop
  with no test harness in facsimile to cover it, and because wolf does not need
  it: `wolf lsp` sends no server→client requests at all, verified in source and
  in the recorded session. Doing it properly is a facsimile PR with facsimile's
  own tests, not a rider on a language registration.
- **The installer count is still a hand-maintained constant.**
  `get_known_servers_count()` returns a literal and `init_known_servers` writes
  through an assumed-shape dummy that never consults `size()`, so the next
  person to add a server hits the same out-of-bounds write. The structural fix
  is `size(servers)`-driven bookkeeping plus a bounds guard; it is a separate,
  self-contained commit and was left out so this series stays about wolf.
- **Python's `"""` and `'''` are unreachable.**
  `string_delimiters = ['"   ', "'   ", '""" ', "''' "]` lists the
  single-character forms first, and `process_string` takes the first match, so
  Python block strings never enter multiline mode. Same root cause as the
  constraint that made wolf's `"""` ordering load-bearing — found while writing
  `load_wolf_syntax`, fixed for wolf only, filed for Python.
- **`shift-alt-l`/`shift-alt-r` are documented but unreachable.**
  `docs`/`help_display_module` advertise them for word selection and no handler
  matches those strings — the same modifier-order bug as formatting, in the
  help text rather than in a `case`.
- **`notify_file_closed` has zero call sites.** Exported, imported by two
  modules, never called. Documents accumulate in the server for the whole
  session, and wiring it as-is would send a bare filename rather than a URI.
- **`didChange` hardcodes `"version": 1`**, so version numbers carry no
  ordering information.
- **Diagnostics have no gutter markers.** The code is commented out and
  `docs/LSP_GUIDE.md` documents behaviour that does not exist. This is where
  D22's work goes to die in this editor, and the highest-value remaining patch
  after the server-request replies.
