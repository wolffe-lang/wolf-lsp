# Editor support matrix

**The claim, and how falsifiable it is.** This file exists to prevent one
specific failure: a README listing eight editors of which two have ever been
run. Every row below names the tier it is verified at, the evidence for that
tier, and — for T1 and T2 — the CI job that re-checks the evidence on every
push. A row that claims a verification it does not have is a bug in this file.

**Last reviewed against wolf pin `70bdd35`, 2026-08-10.**

## The three tiers

| Tier | What "supported" means | How it is verified |
|---|---|---|
| **T1 — automated protocol smoke** | We ship and version a client, or we own its source. A real recorded session exists and replays against a live server. | Recorded transcript + `lspconf onetruth` under that client's profile, plus a CI job that loads the real editor |
| **T2 — automated config check** | We ship a config fragment or a thin extension. Base LSP only. | The config parses / the extension builds, **in CI, by the editor's own tooling**. Protocol behaviour is not exercised by that lane |
| **T3 — documented** | A working recipe. No shipped artefact, best effort. | A human follows the doc on a clean machine once per release and stamps the row |

## The rows

| editor | tier | CI job | evidence | last verified |
|---|---|---|---|---|
| [fackr](../clients/fackr/README.md) | **T1** | `server-lane` (dark: no artifact) | `transcripts/fackr/smoke` · `profiles/fackr.json` (`fackr@496c7e2`) | 2026-08-10, pin `67c977f` |
| [facsimile](../clients/facsimile/README.md) | **T1** | `server-lane` (dark: no artifact) | `transcripts/facsimile/smoke` · `profiles/facsimile.json` (`facsimile@1242ffa`) | 2026-08-10, pin `70bdd35` |
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
  met for any T1 row today: there is no published `wolf` artifact for CI to
  acquire, so `server-lane` is dark everywhere.
- **`emacs` was added to `profiles::REAL_CLIENTS`**, which ls01 §4 fixed at six
  clients. A tracked client whose profile nothing watches for staleness is the
  gap that list exists to close.
- **Zed's build target is `wasm32-wasip2`, not `wasm32-wasip1`.** ls06 §2 names
  wasip1; Zed's `extension_builder.rs` pins
  `const RUST_TARGET: &str = "wasm32-wasip2"`.
- **Helix's `[[grammar]]` block ships commented out** as the sprint requires,
  and so does Zed's `[grammars.wolf]` — for a sharper reason than helix's. helix
  merely gets noisy at startup; **Zed builds every grammar named in the manifest
  at install time**, so a block pointing at the empty `tree-sitter-wolf` fails
  the install and takes the language server down with it.
- **The sprint's helix acceptance test was exercised and reverted.** A fragment
  with a TOML syntax error turns `cargo xtask helix-health` red (4 problems,
  exit 1); adding `language-servers` to the `wolfi` block turns both
  `helix-health` and `config-check` red; dropping one keyword from
  `wolf-mode.el` turns `emacs-check` red; a live `[grammars.wolf]` table turns
  `config-check` red. All four reverted green.

## What no tier gets, on any editor

Every row configures the same binary, `wolf lsp` (D34) — the uniformity is the
point, and it is why a config tier is viable at all. So:

- **Semantic tokens and inlay hints appear in no editor's config here.** Both
  are post-v1 compiler work (s52 non-targets). A client contributing UI for a
  capability the server does not serve produces an editor that looks broken
  rather than one that looks early.
- **No editor post-processes a diagnostic** (D22). The compiler's diagnostics
  are the reviewed artifact; a client that remapped a severity or rewrote a
  message would become a second, unreviewed authority on what the compiler said.
- **`.wolfi` is attached to no language server anywhere.** `wolfi` v0 is a
  binary format and `wolf lsp` discovers modules by `.lu` alone (D32). Four
  clients reached that ruling independently, and `cargo xtask config-check`
  now fails the build if any of them stops honouring it.
- **Syntax highlighting is uneven, and the split is structural.** Neovim and VS
  Code have non-tree-sitter highlighters (`syntax/wolf.vim`,
  `.tmLanguage.json`) and get highlighting today. Helix and Zed highlight
  through tree-sitter only, and `wolffe-lang/tree-sitter-wolf` is a seed commit
  with no `grammar.js` (`b1b2c17`) — so **a `.lu` buffer in Helix or Zed has no
  highlighting at all**. Emacs gets keywords, types and doc comments from
  font-lock, and nothing more.

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
