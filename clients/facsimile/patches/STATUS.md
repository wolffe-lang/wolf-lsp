# facsimile patch series — status

> **UPSTREAMED. This series is in facsimile's trunk.** Re-checked at le05
> against `a121ab3` (v0.35.0): PR1 through PR4c all landed, as ONE squashed
> commit `21e58aa` ("wolf language support: .lu/.wolfi filetypes, wolf lsp
> registration, 50-keyword syntax table, utf-16 position encoding declared,
> installer OOB fix (count 20->21 with the missing increment), unreachable
> alt-shift-f formatting keybinding fixed"). The decomposition below is
> therefore a record of how the work was *organised*, not of how it was
> committed — no PR1..PR4c exists in facsimile's history to point at.
>
> **Beware a number collision.** "PR5" in the table below is this series'
> own fifth group (server-request replies), and it is still not started.
> facsimile's real pull request **#5** is a different and later change set —
> the `wolf-fx01` lane, merge `2f5d5f4` — which fixed capability routing,
> `LocationLink` parsing, bare `CompletionItem[]` parsing and the status
> channel, and closed facsimile#4. Nothing in this table describes it; see
> `../CLIENT.md` and `../../../docs/SERVER-CONSTRAINTS.md`.

The mirror exists so this integration is reproducible from *this* repo even if
the upstream PRs sit unreviewed — which is no longer the situation, but the
reproducibility is still worth keeping. `wolf-integration.diff` is the whole
series against `facsimile@1242ffa`; trunk is now ~120 commits past that base,
so the diff no longer applies cleanly. Per `../CONTRIBUTING-patch.md`, the
remedy when that happens is to re-cut it against the new base rather than let
the mirror rot into fiction — but there is nothing left to re-cut it FOR,
since the content is upstream. It is kept as the historical artifact the
review was done on.

## The series

| PR | files | change | status |
|----|-------|--------|--------|
| **PR1** registration | `src/lsp/lsp_server_manager_module.f90` (`get_language_for_file`, `load_default_configs`), `src/lsp/lsp_client_module.f90` (`detect_language_from_extension`) | `.lu`/`.wolfi` → languageId `wolf`; `add_config(manager, "wolf", "wolf", "wolf lsp", "*.lu,*.wolfi", caps)` with only the four flags wolf serves | **upstreamed** — in trunk via `21e58aa` |
| **PR2** installer entry | `src/lsp/server_detection_module.f90` | count 20 → 21, the missing `i = i + 1` after the fortls block, and a `wolf` row whose `install_cmd` starts with `#` so it routes to the manual-info dialog instead of `sh -c` | **upstreamed** — in trunk via `21e58aa` |
| **PR3** syntax + comments | `src/syntax/syntax_highlighter_module.f90` (`detect_language`, new `load_wolf_syntax`), `src/syntax/comment_syntax_module.f90` (`get_comment_syntax`) | 50 keywords and 4 types generated from the pinned grammar; `//` line comments and no block form; `"""` listed before `"` | **upstreamed** — in trunk via `21e58aa` |
| **PR4a** position encoding | `src/lsp/lsp_protocol_module.f90` | declare `general.positionEncodings: ["utf-16"]` — correct-by-declaration rather than correct-by-default | **upstreamed** — in trunk via `21e58aa` |
| **PR4b** formatting keybinding | `src/commands/command_handler_module.f90` | `case('alt-shift-f', 'shift-alt-f')` — the handler matched a string the input layer never produces, so document formatting could not be invoked at all | **upstreamed** — in trunk via `21e58aa` |
| **PR4c** hot-path hygiene | `src/lsp/lsp_server_manager_module.f90` | drop the unconditional `/tmp/fac_didchange_debug.log` append from `notify_file_changed` | **upstreamed** — in trunk via `21e58aa` |
| **PR5** server-request replies | — | **offered, not written.** See "Deliberately not patched" | not started |
| **PR-test** live smoke | — | **offered, not written.** See `../README.md` §"The CI lane facsimile does not get" | not started |

**No new source files**, so the Makefile's `SOURCES` list was untouched —
deliberately, because it kept the patch reviewable and could not perturb the
build order.

**That build model has since changed, and a re-reader should know.** At
`daa258f` ("Track incremental Fortran module dependencies", after PR #5) the
`SOURCES` order stopped being a recipe-order promise and became real
prerequisites: `Makefile:170-179` chains each object to its predecessor, so a
module change rebuilds every possible consumer after it. A new gate,
`make check-deps` (`Makefile:398-411`), dry-runs a change to
`syntax_highlighter_module.f90` and fails unless `renderer_module.f90` appears
in the plan. It exists because of a near-miss: adding a field to
`syntax_highlighter_t` recompiled the module but not `renderer_module.o`,
which flattened all highlighting until an integration test caught it. Any
future patch to this editor should run `make check-deps` alongside the gates
below.

## Gate results with the series applied

Run in the worktree at `1242ffa` + the diff — a worktree that no longer
exists, so these are a historical record, not a re-runnable claim. The gate
list is also short one entry now: `make check-deps` postdates it.

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

## It is committed now — upstream

This section used to read "Nothing here is committed", describing the working
agreement under which every change sat uncommitted in a dedicated worktree
branch for review, with the diff as the reviewable artifact. That was true
while the series was under review. It landed: `21e58aa` is in facsimile's
trunk and is an ancestor of `a121ab3`. The `wolf-lsp-integration` branch named
above is not visible in any checkout this repo can see, and nothing depends on
it any more.

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
- **Python's block strings were unreachable — FIXED UPSTREAM, 2026-08-27.**
  `string_delimiters` listed the single-character forms first and
  `process_string` takes the first match, so Python block strings never entered
  multiline mode. Same root cause as the constraint that made wolf's ordering
  load-bearing; found while writing `load_wolf_syntax`, fixed for wolf only,
  filed for Python. It has since been fixed for Python too: `d9fafb3` ("List
  python triple-quote delimiters before the single forms") and its sibling
  `c6f8878` ("Exit multiline string mode at the closing delimiter"), both in
  trunk. Those two commits are `docs/UPSTREAM.md`'s PR6 row, which is due a
  `MERGED`.
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
