# Editor support matrix

This file exists to prevent one specific failure: a README listing eight
editors of which two have ever been run. Every row below gives the tier it is
verified at, the evidence for that tier, and (for T1 and T2) the CI job that
re-checks the evidence on every push. A row that claims a verification it does
not have is a bug in this file.

**Last reviewed against wolf pin `6ade878`, 2026-09-05** (le08). That is the
wolf-lang release tag `v0.2.5`, so the pinned version string is the bare
`wolf 0.2.5 (wolfgang, pin 6ade878)`, and `lspconf doctor` reports READY
here. The scripted transcript library was re-recorded at that pin and
`lspconf replay` + `onetruth` ran green under all nine derived profiles
(69 scripted transcripts, 12 samples, zero divergences).

Every one of the 67 inherited transcripts is a header-only diff, and the
only fields that moved are `wolf_pin` and `recorded`, asserted by parsing every
changed record and comparing field by field. So the
server's wire behaviour is byte-identical across `v0.2.4..v0.2.5`, capability
answers included. The two byte transcripts written at le07 did not move either,
which is what le08 set out to check: `hover-byte` and
`completion-byte` both re-recorded header-only, so s137 changed neither the
type display nor completion's answer.

Two NEW transcripts were written at le08 anyway, for the opposite reason le07
had. The le07 pin moved a TYPE and the sweep could not see it. This pin moves
no type at all: wolf-lang v0.2.5's s137 is FIVE CLAUSES that are five builtin
FUNCTIONS: `[os.net.listen.opts]` (`net_listen_with`), `[os.net.wait]`
(`net_wait`), `[os.proc.inherit]` (`os_spawn_with`, `net_adopt_listener`) and
`[os.cpus]` (`os_cpus`); `[os.proc]` is a section header declaring none.
`BUILTIN_TYPES` is byte-identical across the pins at seventeen names; what grew
is `PRELUDE`. Since the server's three type-printing surfaces key on TYPES, the
prediction was "nothing moves", so it was measured:

- `transcripts/requests/hover-net-wait`: five positions in a new vendored
  sample (`net/wait_readiness.lu`, s137's own witness). It found the one thing
  the analogy got wrong. A transcript at le07 pinned that hover on the builtin
  TYPE name `byte` answers `null`; the same was predicted at le08 for a builtin
  FUNCTION name, and measured otherwise. Hover on `net_wait` answers
  `List[int] ! {io}` over
  the range of the whole CALL expression. So the two builtin namespaces are
  asymmetric in the editor: a type name hovers to nothing, a function name
  hovers to the type of the call it heads. It is not a signature (no parameter
  names, no arity, no clause prose), but the error row rides the string, and
  `List[int] ! {io}` is character for character the signature tail
  `[os.net.wait]` writes.
- `transcripts/requests/completion-s137`: completion's absence
  re-verified, and WIDENED. A transcript at le07 pinned that no builtin TYPE
  name is offered. The le08 measurement was a set intersection against the
  pin's own `prelude.rs`: of the
  90 names in `PRELUDE` and the 17 in `BUILTIN_TYPES`, the answer in
  call position offers zero and zero. `print` is not offered.
  `net_listen` is not offered in a document that calls it twice above the
  cursor. That is why `completion-byte` could not have moved: the surface that
  would have shown five new prelude names does not exist. The `.` trigger the
  server advertises still answers empty, now measured on a `List[int]` that
  has a `len`.

`lspconf doctor` is READY, and the archive is whole at this tag. Release
`v0.2.5` (id 382606837) is published with the same four-triple asset set
`v0.2.3` first carried, so the acquire step repaired at le06 needs no change.
The upstream wart reproduces a THIRD time, unchanged and unreaped: the tag has
FOUR releases behind it, the published one plus THREE empty drafts
(`assets=0`, `published=null`), all three stamped eight minutes before the real
one. `v0.2.3` and `v0.2.4` have the identical shape, so wolf-lang#226's
self-publishing release racing is no longer a coincidence. Acquisition resolves
the published one; `gh release list` shows three Draft rows above it, which is
the shape a human misreads. Recorded in `vendor/upstream/PIN`.

Note the tag shape. `v0.2.5` is an ANNOTATED tag: `git rev-parse v0.2.5`
answers the TAG OBJECT (`0bb51c03…`), which peels to the commit `6ade878c…`.
The pin records the peeled commit, because the unpeeled form is a sha no
`wolf --version` will ever print.

## THE SIX CAPTURED SMOKES: ALL SIX, AND THE OBLIGATION IS CLOSED

This obligation has been owed since le01 and named by this file every
sprint since. The captured smokes are the transcripts no script decided (a
real editor's real traffic), so they cannot be re-recorded, only re-CAPTURED by
driving that editor again. At le06 all six sat at pin `70bdd35`; at le07 five
were driven and facsimile was left as a measured red; at le08 all six were
driven at `6ade878`. `lspconf replay` now prints no SKIP line at all: 75
transcripts, zero skipped, where le06 skipped six and le07 skipped one.

| smoke | driven at le08? | how, and what moved |
|---|---|---|
| **nvim** | **RE-CAPTURED** | `nvim --headless` with the documented shim, NVIM **v0.12.5**, 7/7 `smoke.lua` assertions passing while recording. 33 records, **header-only** diff against le07's — bodies byte-identical, so the real editor's real traffic is unchanged at the new pin. Profile unchanged since le07's re-derivation. |
| **fackr** | **RE-CAPTURED** | `cargo test lsp::smoke_wolf::wolf_lsp_corpus_session` in a clean clone at `496c7e2` with `patches/wolf-integration.diff` applied — the series still applies cleanly. 20 records, **header-only**. The user's own fackr worktree was never touched. |
| **helix** | **RE-CAPTURED** | Driven through a pty, helix **25.07.1**, config dropped into a throwaway `XDG_CONFIG_HOME` (the shipped `languages.toml` is what makes `hx --health wolf` resolve the server). 20 records, **header-only**. One driver detail worth keeping: `Space s` opens the symbol PICKER and an open picker swallows the next keys, so the `codeAction` rung vanishes unless the driver sends `Esc` between the two space-mode bindings. |
| **emacs** | **RE-CAPTURED** | `emacs --batch -l clients/emacs/tests/server-test.el`, GNU Emacs **31.1** with built-in eglot **1.24.31**, the suite passing while recording. 24 records, **header-only**. |
| **vscode** | **RE-CAPTURED** | The extension's own test runner against the installed VS Code, **16/16**. Two things measured here, both in `clients/vscode/README.md`: `VSCODE_CLI=1` is **mandatory on macOS** or the lane is a silent no-op (below), and this capture is **not byte-reproducible** (below). |
| **facsimile** | **RE-CAPTURED — and le07's red is withdrawn, not narrowed** | Driven through a pty, `fac` **v0.35.0**. 16 records, **rung for rung identical to the `70bdd35` session**, including BOTH `didChange` rungs and the break/fix round trip le07 reported as unrecordable. Three consecutive runs produce three **byte-identical** transcripts. See below. |

### facsimile: what le07 measured, and what was actually true

Two client-side reasons the session could not be reproduced were recorded at
le07. The first was real and remains true. The second was a wrong conclusion
from a right measurement, and it is withdrawn here.

1. The documented key sequence was stale, and that part stands. At
   `70bdd35` the server did not advertise `completionProvider`; at `982f857`
   and after, it does, and facsimile PR #5 routes on the `initialize` reply,
   so a WORD character now opens a completion popup and every key after it is
   interpreted against a popup that did not exist when the sequence was
   written. The fix is one character wide: break the file with `;` instead of
   a letter. It is not a word character, it produces a clean `E0002`, and
   it is what the nvim and helix smokes already use.

2. "facsimile sends exactly one `didChange` per session, and then stops" is
   FALSE. The le07 probe (five edits three seconds apart) really did produce
   zero notifications, so the measurement was sound; the conclusion drawn from
   it was wrong, and it was about to become a permanent client limitation in
   this file. The mechanism, read out of facsimile's source at `a121ab3` and
   then confirmed on the wire:

   - The flush precedes a BLOCKING read. `app/main.f90:800` calls
     `flush_pending_document_changes` once per main-loop iteration and the very
     next statement is `get_key_input`, which blocks. The debounce check runs
     microseconds after the edit that set `last_change_time`, declines, and the
     loop then parks in the read. A pending change is flushed when the NEXT KEY
     ARRIVES; nothing wakes the loop when the timer expires. An
     edit followed by silence is never sent, however long a driver waits.
   - A buffered burst is coalesced into ONE iteration. The loop after
     `get_key_input` drains every keystroke already buffered
     ("fast typing, paste, or a consumer that fell behind"). A driver that
     writes its key sequence in one `write()` gets one flush no matter how
     many edits it contains, which is what le07 saw.

   So the rule is type, do not paste: one key per `write()`, more than
   `sync_delay` (0.5 s) between them, and a harmless NON-EDIT key after each
   edit to give the loop the iteration in which the debounce can expire. With
   that, the full session records. A third trap cost a run:
   facsimile's `Home` is a SMART home that lands on the first non-blank
   column, so a driver assuming column 0 hovers the `=` and gets `null`.

   Assertions ran against the recording before it was committed, the rule
   the other five smokes follow: the method sequence matches the `70bdd35`
   capture rung for rung, the open publish is clean, the break publish is
   one `E0002`, the fix publish is clean again, hover answers
   `who: str`, `documentSymbol` answers `main`, `formatting` answers `[]`, and
   no server→client request appears.

   What this does NOT change: every constraint in
   [`SERVER-CONSTRAINTS.md`](SERVER-CONSTRAINTS.md) still holds. `handle_request`
   is still an empty stub, there is still no `shutdown`/`exit`, still no
   `$/cancelRequest`, still no `\uXXXX` decoding. Re-verified in the source at
   le08. And `didChange` still carries `"version": 1`, now measured on the
   wire at le08: two distinct edits, two notifications, both version 1.

### The VS Code lane: two properties measured at le08

`VSCODE_CLI=1` is mandatory on macOS, and without it the lane looks like it
worked. `runTest.ts` hands the extension host a `PATH` with the capture
shim's directory first, which is the entire capture mechanism. But VS Code,
launched as a bare Electron binary instead of through its `code` CLI, resolves
the user's LOGIN-SHELL environment in its main process and REPLACES `PATH` with
it, dropping the shim before any extension runs. Measured: the shim's own log
recorded zero invocations while the suite reported 16/16 passing and
printed the correct pinned version, because the real `wolf` on the login `PATH`
answered every probe and served the session. Tests pass, no transcript is
written, `git status` stays clean. That is the third instance in this lane of a
green tick over a no-op, after the two found at le07.

The vscode capture is NOT byte-reproducible, and that is a client property.
`clients/nvim/README.md` records that nvim, recorded three times, produces three
byte-identical files. VS Code does not: three consecutive runs at le08, all
16/16, produced 56, 54 and 58 records. The varying rungs are the ones VS
Code fires on its own timers (`documentSymbol` for the outline, `codeAction`
for the lightbulb, `inlayHint` on scroll), and every test-driven rung
(`didOpen`, `hover`, `formatting`, `publishDiagnostics`, `semanticTokens/*`) has
an identical count across all three. Compare the method multiset. The byte
count varies, so a re-capture differing by a background rung is expected; one
that loses a test-driven rung is a regression.

### The two repairs that lit the VS Code lane

`release-check 3b` has been green since le06, but the VS Code lane behind it
was not: its server half skipped on every run, and the skip looked like an
ordinary "no toolchain at the pin".

1. `src/test/suite/extension.test.ts` compared the wrong string. It matched
   the WHOLE of `wolf --version` against `PIN.version`, but `wolf --version`
   prints two lines (the version, then the lupin pairing) and
   `vendor/upstream/PIN` records the first line, by its own definition. So
   the comparison could never succeed, and this lane had skipped on every pin
   since the day the second line was added, never on the stale binary the
   check exists to catch. It was found at le07 because the skip
   message printed two strings whose first lines were equal.
2. `@vscode/test-electron` 2.5.2 cannot launch a current VS Code on macOS.
   Its darwin branch hardcodes `Visual Studio Code.app/Contents/MacOS/Electron`
   and VS Code stopped shipping that alias after 1.120: the 1.136.1 bundle it
   downloads today contains only `.../MacOS/Code`, so the spawn dies ENOENT
   before a test runs. Symlinking the name back invalidates
   the bundle signature and macOS SIGKILLs the process. Linux resolves a `code`
   script by a different branch, which is why CI never saw it. A
   `WOLF_VSCODE_EXECUTABLE` env override was added to `src/test/runTest.ts` at
   le07 so the lane can run against an installed VS Code; the real repair is a
   dependency bump once upstream handles the rename.

A third detail is recording-only and not committed: VS Code recomputes `PATH`
from the login shell on macOS, which drops the capture shim. `VSCODE_CLI=1` in
the environment suppresses that, and is what let the proxy see the session.

wolf-lsp#7 is CLOSED. Two captured transcripts that leaked a developer's home
directory and could not be re-recorded were waived at le06: `vscode/smoke.jsonl`
seq 39 (a `codeAction` `edit.changes` KEY, captured on a linux box, recorded at
le06 as un-re-capturable from nomad-1) and `emacs/smoke.jsonl` seq 1 (a
`workspaceFolders[0].name` tilde). Both re-captures are clean, and the
exhaustive waiver in `tests/client_recorded.rs` is now empty. The test's
own retirement clause ("a waived file that stops leaking fails this test too")
forced the cleanup.

### What the rows still do NOT claim

The rows below are re-stamped from local captures at le07, and that is all
they claim. D35 and `release-check 3d` want the three-OS claim made from CI,
and a local run cannot make it. `server-lane` was measured green on all three
tier-1 OSes on le06's branch; the row to re-stamp from is still a merge
commit's run.

## The three tiers

| Tier | What "supported" means | How it is verified |
|---|---|---|
| **T1 — automated protocol smoke** | We ship and version a client, or we own its source. A real recorded session exists and replays against a live server. | Recorded transcript + `lspconf onetruth` under that client's profile, plus a CI job that loads the real editor |
| **T2 — automated config check** | We ship a config fragment or a thin extension. Base LSP only. | The config parses / the extension builds, **in CI, by the editor's own tooling**. Protocol behaviour is not exercised by that lane |
| **T3 — documented** | A working recipe. No shipped artefact, best effort. | A human follows the doc on a clean machine once per release and stamps the row |

## The rows

| editor | tier | CI job | evidence | last verified |
|---|---|---|---|---|
| [fackr](../clients/fackr/README.md) | **T1** | `server-lane` (glob fixed at le06) | `transcripts/fackr/smoke` · `profiles/fackr.json` (`fackr@496c7e2`) | **2026-09-05, pin `6ade878`** — RE-CAPTURED at le08; header-only |
| [facsimile](../clients/facsimile/README.md) | **T1** | `server-lane` (glob fixed at le06) | `transcripts/facsimile/smoke` · `profiles/facsimile.json` (`facsimile@1242ffa`) | **2026-09-05, pin `6ade878`, fac v0.35.0** — RE-CAPTURED at le08, rung for rung with the `70bdd35` session; the le01 obligation is CLOSED |
| [Neovim](../clients/nvim/README.md) | **T1** | `nvim-plugin` (3 OS, 14 cases) | `transcripts/nvim/smoke` · `profiles/nvim.json` (`neovim@v0.12.5`) | **2026-09-05, pin `6ade878`, NVIM v0.12.5** — RE-CAPTURED at le08; 7/7, header-only |
| [VS Code](../clients/vscode/README.md) | **T1** | `vscode-extension` (ubuntu, 16 cases) | `transcripts/vscode/smoke` · `profiles/vscode.json` (`vscode@df53daa`) | **2026-09-05, pin `6ade878`** — RE-CAPTURED at le08; 16/16, and the capture is not byte-reproducible (see above) |
| [Helix](../clients/helix/README.md) | **T2** | `helix-config` (3 OS) + `config-check` | `clients/helix/languages.toml` parsed by `hx --health`; `transcripts/helix/smoke` · `profiles/helix.json` (`helix@25.07.1`) | **2026-09-05, pin `6ade878`, helix 25.07.1** — RE-CAPTURED at le08; header-only |
| [Emacs (eglot)](../clients/emacs/README.md) | **T2** | `emacs-mode` (3 OS, 9 cases) + `emacs-check` | `clients/emacs/wolf-mode.el` loaded by `emacs --batch`; `transcripts/emacs/smoke` · `profiles/emacs.json` (`emacs@31.1`, eglot 1.24.31) | **2026-09-05, pin `6ade878`, GNU Emacs 31.1** — RE-CAPTURED at le08; header-only |
| [Zed](../clients/zed/README.md) | **T2** | `zed-extension` (wasm build) + `config-check` | wasm component builds; config statically checked | **wasm build: 2026-08-10.** **Manual run: NEVER — see below** |
| [JetBrains (LSP4IJ)](../clients/jetbrains/README.md) | **T3** | *(none, by design)* | a written recipe | **NEVER — see below** |
| Emacs (lsp-mode) | **T3** | *(none)* | a three-line `lsp-register-client` snippet in `clients/emacs/README.md` | **NEVER — no `lsp-mode` on any machine this repo runs on** |

### The two rows with no stamp, spelled out

Zed has never been run. CI cannot run it: Zed's dev-extension install is a GUI
action (`zed::InstallDevExtension`); its CLI has no `--install-extension` and no
`--dev-extension` flag, and `auto_install_extensions` in `settings.json` covers
published extensions by id and not dev extensions. Nobody has run it by hand
either, because no machine this repository has run on has had Zed installed. So
`profiles/zed.json` and `transcripts/zed/smoke.jsonl` are owed, `lspconf
profiles` names `zed` on every run, and the T2 claim rests on "the wasm builds
and the config is statically consistent". Inventing a profile to shorten that
list is forbidden (`profiles/README.md`), because a fabricated profile produces
a green lane for a client nobody checked.

JetBrains has never been walked end-to-end, for the same reason: no
JetBrains IDE is installed anywhere this repository runs. The recipe was written
from the vendor and LSP4IJ documentation and is unexercised. Per the staleness
rule below, that row renders as *unverified* and will keep doing so until
someone follows it on a clean machine and stamps it.

## Staleness, and why the table can be trusted between releases

A T3 row whose stamp is older than the current release renders as *unverified*.
The table states its own age, so it does not rely on a maintainer remembering
which rows were re-walked. `NEVER` is the value for a row that has never been
walked at all, and it is used above where no date was earned.

A capability profile is stamped with the client version it was read from,
and a profile older than the client it claims to describe is flagged by the same
mechanism. `lspconf profiles` prints the provenance of every profile on every
run (`derived from helix@25.07.1`), and validation *refuses* a `derived` profile
missing its repository, commit or date, which is the shape a fiction would take.

T1 and T2 rows do not depend on anyone's memory, which is what their CI column
is for: their claim is re-checked on every push, and a stale row turns the
build red.

## Maintenance policy

The tier is a promise about maintenance effort, and a promise nobody is paying
for should change.

- T1 breakage blocks a client release. These are daily drivers and the
  editors the book tells readers to use. ls07's release checklist reads this
  file and refuses on a red T1 row.
- T2 breakage files an issue and does not block. A T2 row that stays red
  across two releases is demoted to T3, and the demotion is recorded in
  this file with its date and its reason.
- T3 is docs only. Verified by hand at release time, never gating.

### Promotion

Written down so the table can grow without argument:

- An editor reaches T2 when its config is machine-checkable in CI, by the
  editor's own tooling where that is possible (`hx --health`,
  `emacs --batch`), and by a build of the artefact where it is not (Zed's wasm).
- An editor reaches T1 when a real client session can be recorded and
  replayed headlessly, and `lspconf onetruth` runs under that client's derived
  profile.

Adding an editor to the matrix is a PR that must include its verification lane
at the claimed tier. A row with no lane is a T3 row, with no exceptions: a row
without a lane is the artefact this file exists to prevent.

### Deltas from ls06, recorded here as well as in the campaign closeout

- Emacs was promoted from T3 to T2. ls06 §3 files Emacs under "doc tier,
  verified by a human at release time". Its config turned out to be
  machine-checkable in CI — `emacs --batch` loading `clients/emacs/wolf-mode.el`
  with nine ERT assertions and no `wolf` binary — which is the promotion rule's
  own criterion for T2. Understating a row that a green CI lane verifies would
  make the table lie in the other direction about what is maintained.
- Emacs is *not* promoted to T1, although half the criterion is
  met: a real 23-record eglot session was recorded
  (`transcripts/emacs/smoke.jsonl`) and a profile derived from it. The other
  half, replayed headlessly in CI, is not met, for the same reason it is not
  met for any T1 row today: no published `wolf` artifact matches the current
  pin, so `server-lane` is dark everywhere. (wolf-lang now releases: v0.1.0
  "wolfgang", tagged at `94aa69d`. But the pin has moved past it; the lane
  lights when a release lands at, or the pin returns to, a published sha.)
- `emacs` was added to `profiles::REAL_CLIENTS`, which ls01 §4 fixed at six
  clients. A tracked client whose profile nothing watches for staleness is the
  gap that list exists to close.
- Zed's build target is `wasm32-wasip2`, not `wasm32-wasip1`. ls06 §2 names
  wasip1; Zed's `extension_builder.rs` pins
  `const RUST_TARGET: &str = "wasm32-wasip2"`.
- Helix's `[[grammar]]` block and Zed's `[grammars.wolf]` are LIVE as of
  le02, re-pinned at le04 to tree-sitter-wolf rev `bba5274` (the le04
  branch head, with the `\u{…}` escape bounded at one to six hex digits per
  v0.2.1's `UNI_ESC`; the integrator re-pins on merge/tag), and `config-check`
  now holds the two spellings of the rev equal. They shipped commented out while
  `tree-sitter-wolf` was an empty scaffold: helix only got noisy at startup,
  but Zed builds every grammar named in the manifest at install time, so a
  block pointing at an empty repo failed the install and took the language
  server down with it. That hazard is why the pin points at a rev with a
  committed, CI-verified `src/parser.c`. le02 also added `grammar = "wolf"` and
  `highlights.scm` to `clients/zed/languages/wolf/`, and helix users copy
  tree-sitter-wolf's `queries/*.scm` to `runtime/queries/wolf/`.
- The sprint's helix acceptance test was exercised and reverted. A fragment
  with a TOML syntax error turns `cargo xtask helix-health` red (4 problems,
  exit 1); adding `language-servers` to the `wolfi` block turns both
  `helix-health` and `config-check` red; dropping one keyword from
  `wolf-mode.el` turns `emacs-check` red; a live `[grammars.wolf]` table turns
  `config-check` red. All four reverted green.

## What the server serves, and which transcript pins it per client (s133)

The tier table says how each EDITOR is verified; this one says which
CAPABILITIES the pinned server answers, and names the transcript that pins the
answer's shape under each maintained client's own declarations. A row here is
served on merit: the server does not consult the client's capability
document to decide whether to answer, only to decide the SHAPE (`linkSupport`,
`workspaceEdit.documentChanges`). "not served" rows answer `-32601`
(`transcripts/lifecycle/unknown-method`).

| capability | state | evidence per client | CI job |
|---|---|---|---|
| diagnostics, hover, documentSymbol, formatting, codeAction | served (s52) | `transcripts/{diagnostics,requests}/*`; hover's TYPE DISPLAY additionally pinned at le07 by `transcripts/requests/hover-byte` — the surface v0.2.4's `byte` type actually moved — and at le08 by `transcripts/requests/hover-net-wait`, which found the two builtin namespaces ASYMMETRIC: hover on a builtin TYPE name answers `null`, hover on a builtin FUNCTION name answers the type of the call it heads (`net_wait` → `List[int] ! {io}`, the error row included, over the whole call expression). Not a signature: no parameter names, no arity, no clause prose | `server-lane` |
| completion | **served (s122), and pinned by a transcript for the first time at le07** — with two findings. **(1) It offers no builtin TYPE name at all.** In type position (inside `List[byte]`'s argument, the one place a type is the only legal completion) the answer is the locals in scope, the file's functions, and all FIFTY reserved keywords — and not `byte`, `int`, `str` or `bool`. A user annotating a type in any editor is offered `while` and `spawn` and never the type they are annotating with. **(2) `.` is advertised and answers nothing.** The server declares `completionProvider.triggerCharacters: ["."]`, so every client fires a request on every dot, and member completion returns an EMPTY list. Both are upstream; the transcript records the item set whole so a fix shows as a diff. **le08 widened finding (1) and gave it a number.** It is not that builtin TYPES are missing from completion — it is that the PRELUDE IS MISSING ENTIRELY. Measured by set intersection against the pin's own `prelude.rs` rather than by reading the list: of the **90** names in `PRELUDE` and the **17** in `BUILTIN_TYPES`, the call-position answer offers **zero and zero**. `print` is not offered; `net_listen` is not offered in a document that calls it twice above the cursor. That is also the explanation for a non-event — s137 added five names to `PRELUDE` and `completion-byte` re-recorded header-only, because the surface that would have shown them does not exist. Finding (2) restated at le08 on a `List[int]` that demonstrably has a `len`, through `?` and without it: still empty both ways. **And the row itself was the bug this file exists to prevent**: it cited `transcripts/requests/*` from s133 while NO transcript had ever sent `textDocument/completion` — the word appeared only inside `initialize` capability blocks. | `transcripts/requests/completion-byte`, `transcripts/requests/completion-s137` | `server-lane` |
| `textDocument/definition` | **served (s133)** — `LocationLink[]` to fackr, facsimile, nvim, vscode, emacs (they declare `linkSupport`), `Location[]` to helix | `transcripts/navigation/definition-<client>.jsonl` | `server-lane` |
| `textDocument/references` | **served (s133)** — package-wide, `includeDeclaration` honored, (file, offset) order | `transcripts/navigation/references-<client>.jsonl` | `server-lane` |
| `textDocument/rename` + `prepareRename` | **served (s133)** — `documentChanges` to fackr, facsimile, vscode, helix, emacs, the `changes` map to nvim; refusals by name as `-32803` (`docs/COMPAT.md`) | `transcripts/navigation/rename-<client>.jsonl` | `server-lane` |
| `textDocument/signatureHelp` | **served (s134, at the pin since le06)** — the declared parameters with label offsets, the active parameter by commas, the return type, the `///` doc as markdown to fackr, nvim, vscode, helix, emacs (they list it) and plain text to facsimile (declares no `signatureHelp` at all and asks anyway — answered on merit) | `transcripts/annotate/signatureHelp-<client>.jsonl` | `server-lane` |
| `textDocument/semanticTokens/full` + `/range` | **served (s134, at the pin since le06)** — a closed legend of eight types (`namespace type parameter variable property enumMember function keyword`) and two modifiers (`declaration readonly`), columns in the negotiated encoding; no delta (`-32601` by name) | `transcripts/annotate/semanticTokens-<client>.jsonl` | `server-lane` |
| `textDocument/inlayHint` | **served (s134, at the pin since le06)** — inferred binder types, parameter names at resolved calls; each class off through `initializationOptions.inlayHints.{types,parameterNames}`; whether hints SHOW is the client's toggle (off by default in nvim and helix, a setting in vscode) | `transcripts/annotate/inlayHint-<client>.jsonl` | `server-lane` |
| semantic-token deltas, type definition, workspace symbols, range formatting, pull diagnostics | not served | `transcripts/lifecycle/unknown-method.jsonl` | `server-lane` |

The s134 rows are pinned evidence as of le06. They were recorded against
the wolf-lang `s134` BRANCH binary (a stamped build printing the pinned
string, via `WOLF_BIN`, the pin UNMOVED, the same posture le04 took for
s133's navigation set), and they predicted a header-only diff at the re-pin.
Measured: all eighteen `annotate/*` files re-recorded at `3befc3e` with one
changed line, and the `initialize` answer still lists
`signatureHelpProvider`, `semanticTokensProvider` and `inlayHintProvider`.
`lifecycle/unknown-method` stays targeted at what is still absent
(`typeDefinition`, `semanticTokens/full/delta`), because a probe of a served
method proves nothing. The same held for s133's eighteen `navigation/*` files
at le05, which is now two consecutive branch-recorded sets that re-pinned
without moving.

Where the annotations stop is the CLIENT. The vscode extension contributes
`semanticTokenScopes` as of le06, so the server's eight token types and two
modifiers fall back to scopes a theme already colours instead of to nothing.
nvim's and helix's inlay hints stay off by default, which is each editor's own
default (`vim.lsp.inlay_hint.enable()` is opt-in; helix's `inlay-hints` display
setting is off) and not a gap in this repository; neither is turned on for a
user here.

A client's own gate can hide a served row, and facsimile's no longer does. At
le04 facsimile's static capability table (`caps(CAP_…)`) was recorded as
declining definition/references/rename before asking the server, and as
declaring `linkSupport: true` while parsing only `Location[]`; both were filed
as FortranGoingOnForty/facsimile#4. That issue is closed by facsimile PR #5
(merge `2f5d5f4`, in trunk `a121ab3` / v0.35.0). Routing now reads the
server's own advertised capabilities, the `supports_*` fields in
`lsp_server_manager_module.f90`, filled from the `initialize` reply and
consulted by `server_serves()`; the static table is demoted to the floor used
before that reply arrives, so it can no longer gate off something the running
server does serve. `definition_target()` parses `LocationLink` (preferring
`targetSelectionRange`) as well as `Location`, and the completion popup reads
a bare `CompletionItem[]` as well as a `CompletionList`. All three navigation
rows are reachable in that editor now, and not only answerable by the server.

## What no tier gets, on any editor

Every row configures the same binary, `wolf lsp` (D34), which is why a config
tier is viable at all. So:

- Semantic tokens and inlay hints appear in no editor's config here. Both
  are s134's rungs (definition, references and rename were s133's, and are
  served; see the table above). A client contributing UI for a
  capability the server does not serve produces an editor that looks broken.
- No editor post-processes a diagnostic (D22). The compiler's diagnostics
  are the reviewed artifact; a client that remapped a severity or rewrote a
  message would become a second, unreviewed authority on what the compiler said.
- `.wolfi` is attached to no language server anywhere. `wolfi` v0 is a
  binary format and `wolf lsp` discovers modules by `.lu` alone (D32). Four
  clients reached that ruling independently, and `cargo xtask config-check`
  now fails the build if any of them stops honouring it.
- Syntax highlighting is uneven, but the tree-sitter gap is closed. Neovim
  and VS Code have non-tree-sitter highlighters (`syntax/wolf.vim`,
  `.tmLanguage.json`). Helix and Zed highlight through tree-sitter only; since
  le02 `wolffe-lang/tree-sitter-wolf` holds the real grammar (f-string
  interpolation as expression nodes, corpus-gated at zero ERRORs, and since
  le03 char literals, D63 binder groups and struct patterns, with the
  wolf-lang corpus gate at 443 files / zero ERRORs) and both
  clients' grammar blocks are live, so a `.lu` buffer in Helix or Zed highlights
  once the pinned rev is fetched/installed. Emacs still gets keywords, types
  and doc comments from font-lock, and nothing more.

## Which encodings the real clients reach

The derived profiles are what make wolf's position-encoding preference
(utf-8 → utf-16 → utf-32) testable, and `lspconf onetruth` runs every
sample under every one of them.

| client | declares | negotiates |
|---|---|---|
| fackr | `["utf-32"]` | utf-32 |
| facsimile | `["utf-16"]` | utf-16 |
| VS Code | `["utf-16"]` (hardcoded; **throws** on any other answer) | utf-16 |
| Neovim | `["utf-8", "utf-16", "utf-32"]` | utf-8 |
| Helix | `["utf-8", "utf-32", "utf-16"]` | utf-8 |
| Emacs (eglot) | `["utf-32", "utf-8", "utf-16"]` | utf-8 |
| Zed | *unknown — no session recorded* | *unknown* |

Emacs is the interesting addition: eglot offers utf-32 first, and wolf still
answers utf-8. That is the first client whose own first preference the server
declines, so it is the one that would notice if the server ever started honouring
client order instead of its own.
