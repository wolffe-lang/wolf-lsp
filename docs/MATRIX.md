# Editor support matrix

**The claim, and how falsifiable it is.** This file exists to prevent one
specific failure: a README listing eight editors of which two have ever been
run. Every row below names the tier it is verified at, the evidence for that
tier, and — for T1 and T2 — the CI job that re-checks the evidence on every
push. A row that claims a verification it does not have is a bug in this file.

**Last reviewed against wolf pin `3befc3e`, 2026-09-02** (le06) — the
wolf-lang release tag **`v0.2.3`**, so the pinned version string is the bare
`wolf 0.2.3 (wolfgang, pin 3befc3e)`. The scripted transcript library was
re-recorded at that pin and `lspconf replay` + `onetruth` ran green under all
nine derived profiles (**65** transcripts, 10 samples, zero divergences).
Sixty of the sixty-five diffs are a one-line header whose only moved field is
`wolf_pin`, so the server's wire behaviour is byte-identical across
`v0.2.2..v0.2.3`, capability answers included.

**Five transcripts carry a second diff, and it is this repository's bug, not
the server's.** `Stage::Paths` — the unconditional normalization that keeps a
machine-specific path out of a committed artifact — walked string VALUES only,
and `WorkspaceEdit.changes` is `{ [uri: DocumentUri]: TextEdit[] }`: a rename
or a code-action edit answered to a client that does not declare
`documentChanges` stores its URIs in KEY position and nowhere else. Eight
records across six files shipped a developer's home directory.
`encoding/astral-navigate-{utf8,utf16,utf32}`, `navigation/rename-nvim` and
`requests/code-action-quickfix` are clean at this re-record; the two that are
not are captured sessions that need a re-CAPTURE (**wolf-lsp#7**), and
`tests/client_recorded.rs` now holds the property over every transcript and
every field with those two named in an exhaustive waiver.

**s134's annotating rungs are now recorded at the pin.** The rows below said
"branch-recorded" and predicted a header-only diff at the re-pin. That is what
happened: all eighteen `annotate/*` files re-recorded with only `wolf_pin`
changing, and the `initialize` answer still carries `signatureHelpProvider`,
`semanticTokensProvider` and `inlayHintProvider`. The caveat is retired — the
three rungs are pinned by transcripts recorded against a **tagged release
build**, not a branch build.

The six **captured** editor smokes were NOT re-captured — the same posture
le01 recorded: they keep the pin they were recorded at (`70bdd35`; `67c977f`
for fackr) and `lspconf replay` cannot compare them against the new pin until
someone drives each real editor again. Their rows below stay stamped with the
pin their evidence was actually earned at.

What changed at le06 is how that is REPORTED, and it changed because the lane
finally ran. A script-less transcript at another pin is a named `SKIP:` now,
not a harness error: it cannot be re-recorded by design, so aborting the run
on it meant none of the sixty-five transcripts that ARE at the pin were
replayed at all. `release-check 3b` is consequently green, and `cargo xtask
ci` has no red step for the first time. A SCRIPTED transcript at another pin
is still exit 2 — that one means somebody forgot `lspconf rerecord`.

**The acquire lane: upstream's half is closed and ours is fixed.** Measured
2026-09-02, wolf-lang's `v0.2.3` release is Latest, not Draft, and carries
**four** assets where `v0.2.2` carried three —
`wolf-0.2.3-x86_64-unknown-linux-gnu.tar.gz`,
`wolf-0.2.3-aarch64-unknown-linux-gnu.tar.gz`,
`wolf-0.2.3-aarch64-apple-darwin.tar.gz` and
`wolf-0.2.3-x86_64-pc-windows-msvc.tar.gz`. le05's last open upstream gap
("no linux/aarch64 archive at this tag") is repaired. The remaining half was
**wolf-lsp#3**, a stale glob in this repo: `.github/workflows/ci.yml` asked
for `wolf-<shortsha>-linux-x86_64.tar.gz`, a name `xtask dist` stopped
publishing. le06 fixes the step to ask for `wolf-<version>-<triple>.tar.gz`,
derived from `PIN`, with `--strip-components=1` so the extracted binary lands
where `lsp_harness::locate` looks, and the triple derived per host so all
three tier-1 runners are covered.

**MEASURED: THE LANE LIGHTS.** On this branch the acquire step and `lspconf
doctor` PASSED — the first time `server-lane` has ever resolved a binary — and
on ubuntu the whole lane went green: replay, one-truth, the five server-gated
suites and the seeded fuzz. It found a bug on its first macos run, in a test
that had existed for sprints and had never executed anywhere but a developer's
laptop: `semantics`'s 10 s slow-session waited for the open's diagnostics on a
deadline that knew nothing about the slowness the test itself had injected, and
`didOpen` pays that knob once per query in the analysis pipeline. Fixed on this
branch.

The rows below are still NOT re-stamped from it. A T1 row wants the whole lane
green on all three tier-1 OSes (D35, release-check 3d), and that is a
merge-commit CI result to read, not a branch one to anticipate.

## The three tiers

| Tier | What "supported" means | How it is verified |
|---|---|---|
| **T1 — automated protocol smoke** | We ship and version a client, or we own its source. A real recorded session exists and replays against a live server. | Recorded transcript + `lspconf onetruth` under that client's profile, plus a CI job that loads the real editor |
| **T2 — automated config check** | We ship a config fragment or a thin extension. Base LSP only. | The config parses / the extension builds, **in CI, by the editor's own tooling**. Protocol behaviour is not exercised by that lane |
| **T3 — documented** | A working recipe. No shipped artefact, best effort. | A human follows the doc on a clean machine once per release and stamps the row |

## The rows

| editor | tier | CI job | evidence | last verified |
|---|---|---|---|---|
| [fackr](../clients/fackr/README.md) | **T1** | `server-lane` (glob fixed at le06 — see the header) | `transcripts/fackr/smoke` · `profiles/fackr.json` (`fackr@496c7e2`) | 2026-08-10, pin `67c977f` |
| [facsimile](../clients/facsimile/README.md) | **T1** | `server-lane` (glob fixed at le06 — see the header) | `transcripts/facsimile/smoke` · `profiles/facsimile.json` (`facsimile@1242ffa`) | 2026-08-10, pin `70bdd35` |
| [Neovim](../clients/nvim/README.md) | **T1** | `nvim-plugin` (3 OS, 14 cases) | `transcripts/nvim/smoke` · `profiles/nvim.json` (`neovim@v0.12.4`) | 2026-08-10, pin `70bdd35`, NVIM v0.12.4 |
| [VS Code](../clients/vscode/README.md) | **T1** | `vscode-extension` (ubuntu, 14 cases) | `transcripts/vscode/smoke` · `profiles/vscode.json` (`vscode@df53daa`) | 2026-08-10, pin `70bdd35`, VS Code 1.132.0 |
| [Helix](../clients/helix/README.md) | **T2** | `helix-config` (3 OS) + `config-check` | `clients/helix/languages.toml` parsed by `hx --health`; `transcripts/helix/smoke` · `profiles/helix.json` (`helix@25.07.1`) | 2026-08-10, pin `70bdd35`, helix 25.07.1 |
| [Emacs (eglot)](../clients/emacs/README.md) | **T2** | `emacs-mode` (3 OS, 9 cases) + `emacs-check` | `clients/emacs/wolf-mode.el` loaded by `emacs --batch`; `transcripts/emacs/smoke` · `profiles/emacs.json` (`emacs@30.2`, eglot 1.17.30) | 2026-08-10, pin `70bdd35`, GNU Emacs 30.2 |
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
| diagnostics, hover, documentSymbol, formatting, codeAction | served (s52) | `transcripts/{diagnostics,requests}/*` | `server-lane` |
| completion | served (s122) | `transcripts/requests/*` | `server-lane` |
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
