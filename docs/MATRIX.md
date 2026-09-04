# Editor support matrix

**The claim, and how falsifiable it is.** This file exists to prevent one
specific failure: a README listing eight editors of which two have ever been
run. Every row below names the tier it is verified at, the evidence for that
tier, and — for T1 and T2 — the CI job that re-checks the evidence on every
push. A row that claims a verification it does not have is a bug in this file.

**Last reviewed against wolf pin `982f857`, 2026-09-03** (le07) — the wolf-lang
release tag **`v0.2.4`**, so the pinned version string is the bare
`wolf 0.2.4 (wolfgang, pin 982f857)`, and `lspconf doctor` reports **READY**
here. The scripted transcript library was re-recorded at that pin and
`lspconf replay` + `onetruth` ran green under all nine derived profiles
(**67** transcripts, 11 samples, zero divergences).

**Every one of the 65 inherited transcripts is a header-only diff**, and the
only fields that moved are `wolf_pin` and `recorded` — asserted by walking the
diff and comparing parsed records field by field, not by eyeballing line
counts. So the server's wire behaviour is byte-identical across
`v0.2.3..v0.2.4`, capability answers included.

**That sweep is also why le07 wrote two NEW transcripts rather than trusting
it.** A header-only sweep proves the wire did not move; it proves nothing about
a type the library never bound. wolf-lang v0.2.4's one change this repository
can see is the `byte` type ([type.byte], D72): `str.bytes()` and the eight
byte-producing/consuming builtins answer `List[byte]` where they answered
`List[int]` through v0.2.3 (s136, wolf-lang#231), and hover, inlay hints and
completion detail are the three surfaces that print a type. No sample bound a
byte at all, so the sweep could not have noticed:

- **`transcripts/requests/hover-byte`** — five positions in a new vendored
  sample (`strings/bytes_roundtrip.lu`, the s136 witness): the changed binding
  (`let bs = s.bytes()` hovers `List[byte]`), a BARE `byte` (`for b in bs`), the
  type as WRITTEN rather than inferred (a `List[byte]` parameter),
  [type.byte.cast]'s widening seen from outside (`bs[5] as int` hovers `int`),
  and hover on the builtin type NAME itself, which answers `null`.
- **`transcripts/requests/completion-byte`** — and it does not contain `byte`,
  which is the finding. See the capability table below.

**`lspconf doctor` is READY, and the archive is whole at this tag.** Release
`v0.2.4` is published with the same four-triple asset set `v0.2.3` first
carried, so le06's repaired acquire step needs no change. One upstream wart,
measured and reported rather than worked around: the tag has FOUR releases
behind it — the published one plus THREE empty drafts (wolf-lang#226's
self-publishing release racing; `v0.2.3` has the identical shape). Acquisition
resolves the published one; `gh release list` shows three Draft rows above it,
which is exactly the shape a human misreads. Recorded in `vendor/upstream/PIN`.

## THE SIX CAPTURED SMOKES: FIVE RE-CAPTURED, ONE NARROWED

This obligation has been owed since **le01** and named by this file every
sprint since. The captured smokes are the transcripts no script decided — a
real editor's real traffic — so they cannot be re-recorded, only re-CAPTURED
by driving that editor again. le06 left all six at pin `70bdd35`. le07 drove
**five of the six** on this box and re-captured them at `982f857`. The
`lspconf replay` SKIP list is **six entries shorter by five**.

| smoke | driven? | how, at le07 |
|---|---|---|
| **nvim** | **RE-CAPTURED** | `nvim --headless` with the documented shim, NVIM **v0.12.5**. All 7 `smoke.lua` assertions passed *while recording*. The old capture's `initialize` answer had **no `completionProvider`** — the transcript was materially wrong about what the server serves. Profile RE-DERIVED at v0.12.5: eglot-style drift found, `inlayHint.resolveSupport` moved from `location`/`command` to the dotted `label.location`/`label.tooltip`/`label.command`, and `didChangeWatchedFiles.dynamicRegistration` false → true. |
| **fackr** | **RE-CAPTURED** | `cargo test lsp::smoke_wolf::wolf_lsp_corpus_session` in a clean clone at `496c7e2` with `patches/wolf-integration.diff` applied — which is how the patch series' own README says to reproduce it, and the series still applies cleanly. The user's own fackr worktree was never touched. |
| **helix** | **RE-CAPTURED** | Driven through a pty with the stdlib `pty` module, helix **25.07.1** — the exact version `profiles/helix.json` was derived from, and the captured capability document is **byte-identical** to the profile, so no re-derivation was owed. Every rung of the recorded session reproduced, plus a NEW `textDocument/signatureHelp`: helix fires it on entering insert mode and the server has advertised `signatureHelpProvider` since s134. 9 records match on replay, up from 8. **The `[[grammar]]` and `languages.toml` fragment this repo ships is what made it work** — `hx --health wolf` resolves the server from it. |
| **emacs** | **RE-CAPTURED** | `emacs --batch -l clients/emacs/tests/server-test.el`, GNU Emacs **31.1** with built-in eglot **1.24.31**. All eight `ert-info` sections asserted while recording; 24 records, rung for rung the old session. Profile RE-DERIVED at 31.1 — eglot grew a lot since 30.2: `$streamingDiagnostics`, `callHierarchy`, `diagnostic`, `semanticTokens`, `completion.insertReplaceSupport`, `publishDiagnostics.versionSupport` and — load-bearing for this repo — **`rename.prepareSupport: true`**. |
| **vscode** | **RE-CAPTURED** | The extension's own test runner, against the installed VS Code. **16/16 passing, and the server half of that suite had never run.** Getting there needed two fixes, both in this repo and both described at their site — see the next section. |
| **facsimile** | **NOT re-captured — and this is the one narrow red** | The editor CAN be driven here: `fac` v0.35.0 builds and runs, `pexpect`/`pyte` are present, and the read-only rungs all answer correctly at the new pin (hover → the type, documentSymbol → the one function, formatting → no edits, clean publish on open). What cannot be reproduced is the *recorded session*, for a measured client-side reason. See below. |

### Why facsimile could not be re-captured, precisely

Two independent findings, both in the client and neither in `wolf lsp`:

1. **The documented key sequence is stale, because the server's capability set
   moved under it.** At `70bdd35` the server did not advertise
   `completionProvider`. At `982f857` it does, and facsimile PR #5 routes on
   the `initialize` reply — so the editor now opens a completion popup on the
   very keystroke (`x`) the recorded sequence uses to break the file, and every
   key after it is interpreted against a popup that did not exist when the
   sequence was written.
2. **facsimile sends exactly one `didChange` per session, and then stops.**
   This is the blocker, and it is not a timing artifact of the driver: a probe
   that made **five** separate edits three seconds apart, pumping the input
   loop between each, produced **zero** `didChange` notifications. The
   break/fix round-trip — half the value of the smoke — therefore cannot be
   recorded at all. A transcript missing it would replay green forever while
   covering less than the one it replaced, and `clients/nvim/README.md` states
   the rule this repo follows: *a transcript of a broken session is worse than
   none*. The `70bdd35` capture is kept, and its row keeps the pin it earned.

Both are reported to facsimile. The client mirror was re-read against the
human's trunk at le07 and re-recorded where it had drifted — see
`clients/facsimile/CLIENT.md` and `patches/STATUS.md`.

### The two repairs that lit the VS Code lane

`release-check 3b` has been green since le06, but the VS Code lane behind it
was not: its server half skipped on every run, and the skip looked like an
ordinary "no toolchain at the pin".

1. **`src/test/suite/extension.test.ts` compared the wrong string.** It matched
   the WHOLE of `wolf --version` against `PIN.version` — but `wolf --version`
   prints two lines (the version, then the lupin pairing) and
   `vendor/upstream/PIN` records the **first line**, by its own definition. So
   the comparison could never succeed, and this lane had skipped on every pin
   since the day the second line was added — not on a stale binary, which is
   the only thing the check exists to catch. le07 found it because the skip
   message printed two strings whose first lines were equal.
2. **`@vscode/test-electron` 2.5.2 cannot launch a current VS Code on macOS.**
   Its darwin branch hardcodes `Visual Studio Code.app/Contents/MacOS/Electron`
   and VS Code stopped shipping that alias after 1.120 — the 1.136.1 bundle it
   downloads today contains only `.../MacOS/Code`, so the spawn dies ENOENT
   before a test runs. Symlinking the name back is not a fix: it invalidates
   the bundle signature and macOS SIGKILLs the process. Linux resolves a `code`
   script by a different branch, which is why CI never saw it. le07 adds a
   `WOLF_VSCODE_EXECUTABLE` env override to `src/test/runTest.ts` so the lane
   can run against an installed VS Code; the real repair is a dependency bump
   once upstream handles the rename.

A third detail is recording-only and not committed: VS Code recomputes `PATH`
from the login shell on macOS, which drops the capture shim. `VSCODE_CLI=1` in
the environment suppresses that, and is what let the proxy see the session.

**wolf-lsp#7 is CLOSED.** le06 waived two captured transcripts that leaked a
developer's home directory and could not be re-recorded — `vscode/smoke.jsonl`
seq 39 (a `codeAction` `edit.changes` KEY, captured on a linux box, which le06
recorded as un-re-capturable from nomad-1) and `emacs/smoke.jsonl` seq 1 (a
`workspaceFolders[0].name` tilde). Both re-captures are clean, and the
exhaustive waiver in `tests/client_recorded.rs` is now **empty** — the test's
own retirement clause ("a waived file that stops leaking fails this test too")
is what forced the cleanup, and it worked.

### What the rows still do NOT claim

The rows below are re-stamped from le07's **local** captures, and that is all
they claim. D35 and `release-check 3d` want the three-OS claim made from CI,
and a local run is not CI's. `server-lane` was measured green on all three
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
| [fackr](../clients/fackr/README.md) | **T1** | `server-lane` (glob fixed at le06) | `transcripts/fackr/smoke` · `profiles/fackr.json` (`fackr@496c7e2`) | **2026-09-03, pin `982f857`** — RE-CAPTURED at le07 |
| [facsimile](../clients/facsimile/README.md) | **T1** | `server-lane` (glob fixed at le06) | `transcripts/facsimile/smoke` · `profiles/facsimile.json` (`facsimile@1242ffa`) | 2026-08-10, pin `70bdd35` — **re-capture attempted and refused at le07; see the header** |
| [Neovim](../clients/nvim/README.md) | **T1** | `nvim-plugin` (3 OS, 14 cases) | `transcripts/nvim/smoke` · `profiles/nvim.json` (`neovim@v0.12.5`) | **2026-09-03, pin `982f857`, NVIM v0.12.5** — RE-CAPTURED, profile re-derived |
| [VS Code](../clients/vscode/README.md) | **T1** | `vscode-extension` (ubuntu, 16 cases) | `transcripts/vscode/smoke` · `profiles/vscode.json` (`vscode@df53daa`) | **2026-09-03, pin `982f857`, VS Code 1.120.0** — RE-CAPTURED; 16/16, the server half ran for the first time |
| [Helix](../clients/helix/README.md) | **T2** | `helix-config` (3 OS) + `config-check` | `clients/helix/languages.toml` parsed by `hx --health`; `transcripts/helix/smoke` · `profiles/helix.json` (`helix@25.07.1`) | **2026-09-03, pin `982f857`, helix 25.07.1** — RE-CAPTURED; profile byte-identical, no re-derivation owed |
| [Emacs (eglot)](../clients/emacs/README.md) | **T2** | `emacs-mode` (3 OS, 9 cases) + `emacs-check` | `clients/emacs/wolf-mode.el` loaded by `emacs --batch`; `transcripts/emacs/smoke` · `profiles/emacs.json` (`emacs@31.1`, eglot 1.24.31) | **2026-09-03, pin `982f857`, GNU Emacs 31.1** — RE-CAPTURED, profile re-derived |
| [Zed](../clients/zed/README.md) | **T2** | `zed-extension` (wasm build) + `config-check` | wasm component builds; config statically checked | **wasm build: 2026-08-10.** **Manual run: NEVER — see below** |
| [JetBrains (LSP4IJ)](../clients/jetbrains/README.md) | **T3** | *(none, by design)* | a written recipe | **NEVER — see below** |
| Emacs (lsp-mode) | **T3** | *(none)* | a three-line `lsp-register-client` snippet in `clients/emacs/README.md` | **NEVER — no `lsp-mode` on any machine this repo runs on** |

### The two rows with no stamp, spelled out

**Zed has never been run.** Not by CI — Zed's dev-extension install is a GUI
action (`zed::InstallDevExtension`); its CLI has no `--install-extension` and no
`--dev-extension` flag, and `auto_install_extensions` in `settings.json` covers
published extensions by id and not dev extensions. And not by hand — no machine
this repository has run on has had Zed installed. So `profiles/zed.json` and
`transcripts/zed/smoke.jsonl` are **owed**, `lspconf profiles` names `zed` on
every run, and the T2 claim rests entirely on "the wasm builds and the config is
statically consistent". Inventing a profile to shorten that list is specifically
forbidden (`profiles/README.md`), because a fabricated profile produces a green
lane for a client nobody checked.

**JetBrains has never been walked end-to-end**, for the same reason: no
JetBrains IDE is installed anywhere this repository runs. The recipe was written
from the vendor and LSP4IJ documentation and is unexercised. Per the staleness
rule below, that row renders as *unverified* and will keep doing so until
someone follows it on a clean machine and stamps it.

## Staleness, and why the table can be trusted between releases

**A T3 row whose stamp is older than the current release renders as
*unverified*.** The table tells the truth about its own age rather than relying
on a maintainer remembering which rows were re-walked. `NEVER` is the honest
value for a row that has never been walked at all, and it is used above rather
than a date nobody earned.

**A capability profile is stamped with the client version it was read from,**
and a profile older than the client it claims to describe is flagged by the same
mechanism. `lspconf profiles` prints the provenance of every profile on every
run (`derived from helix@25.07.1`), and validation *refuses* a `derived` profile
missing its repository, commit or date — that being exactly the shape a fiction
would take.

**T1 and T2 rows do not depend on anyone's memory**, which is the whole point of
their CI column: their claim is re-checked on every push, and a stale row is a
red build rather than an old date.

## Maintenance policy

The tier is a promise about maintenance effort, and a promise nobody is paying
for should change, visibly.

- **T1 breakage blocks a client release.** These are daily drivers and the
  editors the book tells readers to use. ls07's release checklist reads this
  file and refuses on a red T1 row.
- **T2 breakage files an issue and does not block.** A T2 row that stays red
  across **two** releases is **demoted to T3**, and the demotion is recorded in
  this file with its date and its reason. A tier nobody is maintaining should
  say so.
- **T3 is docs only.** Verified by hand at release time, never gating.

### Promotion

Written down so the table can grow without argument:

- An editor reaches **T2** when its config is machine-checkable in CI — by the
  editor's own tooling where that is possible (`hx --health`,
  `emacs --batch`), and by a build of the artefact where it is not (Zed's wasm).
- An editor reaches **T1** when a real client session can be **recorded and
  replayed headlessly**, and `lspconf onetruth` runs under that client's derived
  profile.

**Adding an editor to the matrix is a PR that must include its verification lane
at the claimed tier. A row with no lane is a T3 row.** No exceptions: a row
without a lane is the exact artefact this file exists to prevent.

### Deltas from ls06, recorded here as well as in the campaign closeout

- **Emacs was promoted from T3 to T2.** ls06 §3 files Emacs under "doc tier,
  verified by a human at release time". Its config turned out to be
  machine-checkable in CI — `emacs --batch` loading `clients/emacs/wolf-mode.el`
  with nine ERT assertions and no `wolf` binary — which is the promotion rule's
  own criterion for T2. Understating a row that a green CI lane verifies would
  make the table lie in the other direction about what is maintained.
- **Emacs is deliberately *not* promoted to T1**, although half the criterion is
  met: a real 23-record eglot session was recorded
  (`transcripts/emacs/smoke.jsonl`) and a profile derived from it. The other
  half — replayed headlessly in CI — is not met, for the same reason it is not
  met for any T1 row today: no published `wolf` artifact matches the current
  pin, so `server-lane` is dark everywhere. (wolf-lang now releases — v0.1.0
  "wolfgang", tagged at `94aa69d` — but the pin has moved past it; the lane
  lights when a release lands at, or the pin returns to, a published sha.)
- **`emacs` was added to `profiles::REAL_CLIENTS`**, which ls01 §4 fixed at six
  clients. A tracked client whose profile nothing watches for staleness is the
  gap that list exists to close.
- **Zed's build target is `wasm32-wasip2`, not `wasm32-wasip1`.** ls06 §2 names
  wasip1; Zed's `extension_builder.rs` pins
  `const RUST_TARGET: &str = "wasm32-wasip2"`.
- **Helix's `[[grammar]]` block and Zed's `[grammars.wolf]` are LIVE as of
  le02**, re-pinned at le04 to tree-sitter-wolf rev `bba5274` (the le04
  branch head — the `\u{…}` escape bounded at one to six hex digits per
  v0.2.1's `UNI_ESC`; the integrator re-pins on merge/tag), and `config-check`
  now holds the two spellings of the rev equal. They shipped commented out while
  `tree-sitter-wolf` was an empty scaffold — helix merely got noisy at startup,
  but **Zed builds every grammar named in the manifest at install time**, so a
  block pointing at an empty repo failed the install and took the language
  server down with it. That hazard is why the pin points at a rev with a
  committed, CI-verified `src/parser.c`. le02 also added `grammar = "wolf"` and
  `highlights.scm` to `clients/zed/languages/wolf/`, and helix users copy
  tree-sitter-wolf's `queries/*.scm` to `runtime/queries/wolf/`.
- **The sprint's helix acceptance test was exercised and reverted.** A fragment
  with a TOML syntax error turns `cargo xtask helix-health` red (4 problems,
  exit 1); adding `language-servers` to the `wolfi` block turns both
  `helix-health` and `config-check` red; dropping one keyword from
  `wolf-mode.el` turns `emacs-check` red; a live `[grammars.wolf]` table turns
  `config-check` red. All four reverted green.

## What the server serves, and which transcript pins it per client (s133)

The tier table says how each EDITOR is verified; this one says which
CAPABILITIES the pinned server answers, and names the transcript that pins the
answer's shape under each maintained client's own declarations. A row here is
served on merit — the server does not consult the client's capability
document to decide whether to answer, only to decide the SHAPE (`linkSupport`,
`workspaceEdit.documentChanges`). "not served" rows answer `-32601` by name
(`transcripts/lifecycle/unknown-method`).

| capability | state | evidence per client | CI job |
|---|---|---|---|
| diagnostics, hover, documentSymbol, formatting, codeAction | served (s52) | `transcripts/{diagnostics,requests}/*`; hover's TYPE DISPLAY additionally pinned at le07 by `transcripts/requests/hover-byte` — the surface v0.2.4's `byte` type actually moved | `server-lane` |
| completion | **served (s122), and pinned by a transcript for the first time at le07** — with two findings. **(1) It offers no builtin TYPE name at all.** In type position (inside `List[byte]`'s argument, the one place a type is the only legal completion) the answer is the locals in scope, the file's functions, and all FIFTY reserved keywords — and not `byte`, `int`, `str` or `bool`. A user annotating a type in any editor is offered `while` and `spawn` and never the type they are annotating with. **(2) `.` is advertised and answers nothing.** The server declares `completionProvider.triggerCharacters: ["."]`, so every client fires a request on every dot, and member completion returns an EMPTY list. Both are upstream; the transcript records the item set whole so a fix shows as a diff. **And the row itself was the bug this file exists to prevent**: it cited `transcripts/requests/*` from s133 while NO transcript had ever sent `textDocument/completion` — the word appeared only inside `initialize` capability blocks. | `transcripts/requests/completion-byte` | `server-lane` |
| `textDocument/definition` | **served (s133)** — `LocationLink[]` to fackr, facsimile, nvim, vscode, emacs (they declare `linkSupport`), `Location[]` to helix | `transcripts/navigation/definition-<client>.jsonl` | `server-lane` |
| `textDocument/references` | **served (s133)** — package-wide, `includeDeclaration` honored, (file, offset) order | `transcripts/navigation/references-<client>.jsonl` | `server-lane` |
| `textDocument/rename` + `prepareRename` | **served (s133)** — `documentChanges` to fackr, facsimile, vscode, helix, emacs, the `changes` map to nvim; refusals by name as `-32803` (`docs/COMPAT.md`) | `transcripts/navigation/rename-<client>.jsonl` | `server-lane` |
| `textDocument/signatureHelp` | **served (s134, at the pin since le06)** — the declared parameters with label offsets, the active parameter by commas, the return type, the `///` doc as markdown to fackr, nvim, vscode, helix, emacs (they list it) and plain text to facsimile (declares no `signatureHelp` at all and asks anyway — answered on merit) | `transcripts/annotate/signatureHelp-<client>.jsonl` | `server-lane` |
| `textDocument/semanticTokens/full` + `/range` | **served (s134, at the pin since le06)** — a closed legend of eight types (`namespace type parameter variable property enumMember function keyword`) and two modifiers (`declaration readonly`), columns in the negotiated encoding; no delta (`-32601` by name) | `transcripts/annotate/semanticTokens-<client>.jsonl` | `server-lane` |
| `textDocument/inlayHint` | **served (s134, at the pin since le06)** — inferred binder types, parameter names at resolved calls; each class off through `initializationOptions.inlayHints.{types,parameterNames}`; whether hints SHOW is the client's toggle (off by default in nvim and helix, a setting in vscode) | `transcripts/annotate/inlayHint-<client>.jsonl` | `server-lane` |
| semantic-token deltas, type definition, workspace symbols, range formatting, pull diagnostics | not served | `transcripts/lifecycle/unknown-method.jsonl` | `server-lane` |

**The s134 rows are pinned evidence as of le06.** They were recorded against
the wolf-lang `s134` BRANCH binary (a stamped build printing the pinned
string, via `WOLF_BIN`, the pin UNMOVED — the same posture le04 took for
s133's navigation set), and they predicted a header-only diff at the re-pin.
Measured: all eighteen `annotate/*` files re-recorded at `3befc3e` with
exactly one changed line, and the `initialize` answer still carries
`signatureHelpProvider`, `semanticTokensProvider` and `inlayHintProvider`.
`lifecycle/unknown-method` stays targeted at what is still absent
(`typeDefinition`, `semanticTokens/full/delta`), because a probe of a served
method proves nothing. The same held for s133's eighteen `navigation/*` files
at le05, which is now two consecutive branch-recorded sets that re-pinned
without moving.

**Where the annotations stop is the CLIENT, and that is stated rather than
implied.** The vscode extension contributes `semanticTokenScopes` as of le06,
so the server's eight token types and two modifiers fall back to scopes a
theme already colours instead of to nothing. nvim's and helix's inlay hints
stay **off by default** — that is each editor's own default
(`vim.lsp.inlay_hint.enable()` is opt-in; helix's `inlay-hints` display
setting is off), not a gap in this repository, and neither is turned on for a
user here.

**A client's own gate can hide a served row — and facsimile's no longer
does.** le04 recorded that facsimile's static capability table (`caps(CAP_…)`)
declined definition/references/rename before asking the server, and that it
declared `linkSupport: true` while parsing only `Location[]`; both were filed
as FortranGoingOnForty/facsimile#4. **That issue is closed by facsimile PR #5**
(merge `2f5d5f4`, in trunk `a121ab3` / v0.35.0). Routing now reads the
server's own advertised capabilities — `lsp_server_manager_module.f90`'s
`supports_*` fields, filled from the `initialize` reply and consulted by
`server_serves()` — and the static table is demoted to the floor used before
that reply arrives, so it can no longer gate off something the running server
does serve. `definition_target()` parses `LocationLink` (preferring
`targetSelectionRange`) as well as `Location`, and the completion popup reads
a bare `CompletionItem[]` as well as a `CompletionList`. All three navigation
rows are reachable in that editor now, not just answerable by the server.

## What no tier gets, on any editor

Every row configures the same binary, `wolf lsp` (D34) — the uniformity is the
point, and it is why a config tier is viable at all. So:

- **Semantic tokens and inlay hints appear in no editor's config here.** Both
  are s134's rungs (definition, references and rename were s133's, and are
  served — see the table above). A client contributing UI for a
  capability the server does not serve produces an editor that looks broken
  rather than one that looks early.
- **No editor post-processes a diagnostic** (D22). The compiler's diagnostics
  are the reviewed artifact; a client that remapped a severity or rewrote a
  message would become a second, unreviewed authority on what the compiler said.
- **`.wolfi` is attached to no language server anywhere.** `wolfi` v0 is a
  binary format and `wolf lsp` discovers modules by `.lu` alone (D32). Four
  clients reached that ruling independently, and `cargo xtask config-check`
  now fails the build if any of them stops honouring it.
- **Syntax highlighting is uneven, but the tree-sitter gap is closed.** Neovim
  and VS Code have non-tree-sitter highlighters (`syntax/wolf.vim`,
  `.tmLanguage.json`). Helix and Zed highlight through tree-sitter only; since
  le02 `wolffe-lang/tree-sitter-wolf` carries the real grammar (f-string
  interpolation as expression nodes, corpus-gated at zero ERRORs — and since
  le03 char literals, D63 binder groups and struct patterns, with the
  wolf-lang corpus gate at 443 files / zero ERRORs) and both
  clients' grammar blocks are live — a `.lu` buffer in Helix or Zed highlights
  once the pinned rev is fetched/installed. Emacs still gets keywords, types
  and doc comments from font-lock, and nothing more.

## Which encodings the real clients reach

Not decoration: the derived profiles are what make wolf's position-encoding
preference (utf-8 → utf-16 → utf-32) testable, and `lspconf onetruth` runs every
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

Emacs is the interesting addition: eglot offers **utf-32 first**, and wolf still
answers utf-8. That is the first client whose own first preference the server
declines, so it is the one that would notice if the server ever started honouring
client order instead of its own.
