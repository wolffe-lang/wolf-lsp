# The six smokes, re-captured at 0.2.32 on one linux host (lane tl26)

wolf-lsp#26: the six captured editor smokes
(`transcripts/{emacs,fackr,facsimile,helix,nvim,vscode}/smoke.jsonl`) were
last captured at `30731a6` (v0.2.14, tl07) on nomad-1 (darwin arm64), and
`lspconf replay` has skipped them by name at every pin since. This file is
lane tl26's contract and record: all six driven again at the pin `72de141f`
(wolf 0.2.32) on **one host, kasumi (linux x86-64)**, every moved field
classified as **host**, **editor build** or **wolf pin**, with the evidence.
Contract: planning `sprints/lsp/tl26-the-six-smokes.md` (`ca483a3`); rules
`sprints/crawl-54.md` down to `wave-45.md`; inputs wolf-lsp#26, #18, #45,
tl07's and tl03's capture method (`docs/MATRIX.md`).

Order: §1–§3 are committed before any transcript is re-captured; §3 is not
edited after this commit. Before it, the lane acquired the archive by digest,
built `lspconf`, and ran one read-only measurement (§2, the old traffic at
the new pin), because a prediction that does not know what the server
answers is a guess.

## 1. Forbidden, absolutely

- No change to the server's behaviour, the harness's matching or the
  normalizer to make a transcript pass; a moved record that is a server change
  is a finding, listed in §4. No hand edit of a captured transcript, except
  the plant, which is reverted.
- No `rm` outside `~/lanes/tl26/`; no `git add -A`; nothing under `~/.claude`.
- No build on nomad-1. Every `cargo`, `lspconf`, `fpm` and editor run is on
  kasumi under `~/lanes/tl26/`, `CARGO_INCREMENTAL=0`. Lanes install no host
  tool: the editors are the ones the orchestrator installed, read by version.
- The maintainer's own fackr worktree on nomad-1 is never touched: fackr is
  cloned from `github.com/tenseleyFlow/fackr` on kasumi.
- No merge, no tag, no issue filed or closed; the branch is asserted before
  every commit; no attribution trailers.
- The pin is the release by digest, read twice: wolf 0.2.32 is `v0.2.32` at
  `72de141f`, the x86_64-unknown-linux-gnu archive of release 416327027.
- kill by recorded pid only; `setsid` with a pid file; never host ports
  8080/8088.

## 2. Inputs, verified

- **The server.** `wolf-0.2.32-x86_64-unknown-linux-gnu.tar.gz`, downloaded on
  kasumi: API digest, `sha256sum` and `openssl dgst` all
  `4c37f9c5dc3d8830c06810948027d38bdbc375dc5524ab23127b0ada0907d851`,
  80,092,071 bytes; `wolf` member `81bdf8d4…` (= `vendor/upstream/PIN`);
  `wolf --version` → `wolf 0.2.32 (wolfgang, pin 72de141)` / `paired with
  lupin 0.1.54 (reference interpreter), pin 294d626`; `std/` 58 files,
  `STD-REV` `d37c911f…`, list digest `97647fb1…` (= PIN). The whole archive
  unpacked as `.wolf-bin/`, as CI does. `lspconf doctor`: READY at 72de141.
- **wolf-lsp** trunk `cf9345a`, cloned on kasumi into a directory literally
  named `wolf-lsp` (`~/lanes/tl26/wolf-lsp`), because helix rides the
  checkout's basename in `workspaceFolders[0].name` (wolf-lsp#18, tl07's
  fourth trap) and `capture` writes into the repo root compiled into the
  binary.
- **Baseline replay at the pin**: `lspconf --require-server replay` exit 0,
  73 ok, 0 mismatched, the six SKIPPED by name.
- **The old traffic at the new pin (read-only, scratch).** Each committed
  smoke copied to `~/lanes/tl26/b0/old/`, its header's `wolf_pin` swapped
  from `30731a6` to `72de141f` and nothing else, and replayed against the
  0.2.32 server: **all six match** — emacs 10, fackr 9, facsimile 7, helix 9,
  nvim 15, vscode 26 records, every teardown as recorded. So the server's
  answer to the exact traffic captured at 0.2.14 has not moved in any
  record replay can match, across ten pins and the v0.2.29 server change.
- **The editors on kasumi** (CachyOS, linux 7.2.8, x86-64), read by the lane:

  | editor | at tl07 (nomad-1, darwin arm64) | on kasumi now |
  |---|---|---|
  | nvim | NVIM v0.12.5, official macOS release tarball (`clientInfo.version` `0.12.5+v0.12.5`) | NVIM v0.12.5, `neovim 0.12.5-1.1` (distribution package, `Build type: RelWithDebInfo`) |
  | helix | 25.07.1, `clientInfo.version` `25.07.1 (a05c151b)` | `helix 25.07.1 (4746c276)`, package `helix 25.07.1-2.1`, `/usr/lib/helix/hx` |
  | emacs | GNU Emacs 31.1, built-in eglot 1.24.31 | GNU Emacs 31.1, `emacs-nox 31.1-2`, built-in eglot `;; Version: 1.24.31`, no `~/.emacs.d/elpa` |
  | fackr | `cargo test` in a clone at `496c7e2` + `wolf-integration.diff`, `clientInfo` `fackr 1.2.1` | the same procedure, a clone from GitHub on kasumi, rustc 1.97.1 (the installed `~/.cargo/bin/fackr` v1.2.1 at trunk `e9ed924` is not what the smoke runs) |
  | facsimile | `fac` v0.35.0 built with `fpm` from a clone at `a121ab3` | the same: `fpm` from a clone at `a121ab3` (the packaged `/usr/bin/fac`, `facsimile 0.35.0-1`, recorded but not used, so the build stays fixed and only the host moves) |
  | vscode | VS Code 1.120.0, macOS, `VSCODE_CLI=1` | `code` 1.136.1 (`a44adf7f…`, x64, `visual-studio-code-bin 1.136.1-1`); **no display, no `xvfb-run`** |

## 3. Prediction

A field is classified **wolf pin** when it moves with the server or the
header's pin; **editor build** when the same editor version, built or
packaged differently, sends it differently; **host** when it follows the
operating system, filesystem or checkout layout. Every count is protocol
records (lines minus the header).

| # | prediction |
|---|---|
| P1 | **wolf pin: header only.** On every re-captured smoke the header's `wolf_pin` (`30731a6` → `72de141f`) and `recorded` move, and **no server→client record moves** in any field (the §2 replay of the old traffic already matched at this pin; the records replay does not match are client-timed or carry nothing the server decides beyond what was matched). |
| P2 | **nvim**: 32 records, the same method sequence; one c2s field moves, `clientInfo.version` `0.12.5+v0.12.5` → a string without the `+v0.12.5` build stamp (the distribution package, not the release tarball): **editor build**. Warm runs byte-identical to each other. |
| P3 | **helix**: 19 records, the same sequence; `clientInfo.version` `25.07.1 (a05c151b)` → `25.07.1 (4746c276)`: **editor build** (the same tag, a different build hash). `workspaceFolders[0].name` stays `wolf-lsp` (the checkout is named for it). |
| P4 | **emacs**: 23 records, **header-only** (the same Emacs and eglot version on both hosts; `rootPath`/`workspaceFolders[].name` are elided paths, `$REPO/`). |
| P5 | **fackr**: 19 records, **header-only**; `git apply --check` of the series exits 0 at `496c7e2`. At fackr trunk `e9ed924` the series is checked too, without changing the recorded base: predicted to **not** apply cleanly (the series touches `src/lsp/*`, which trunk has moved since April). |
| P6 | **facsimile**: 15 records, rung for rung, **header-only** (the same `a121ab3` build; `clientInfo` is `fac 0.1.0`, a constant). |
| P7 | **vscode: not captured.** The extension's runner needs a display on linux (`xvfb-run`), kasumi has none, and the lane installs nothing; if Electron cannot be driven with no display at all, the transcript stays at `30731a6`, replay covers **78 of the 79 files (73 scripted + 5 captured), 1 SKIPPED**, and §4 says exactly what it needs. |
| P8 | **replay at the end**: 79 files, 78 ok, 0 mismatched, 1 skipped (vscode), exit 0, locally and in CI on three OSes; nothing besides the five transcripts, `profiles/` unchanged, MATRIX/COMPAT/compat rows. |
| P9 | **No host field moves at all** on the five: every path the editors send is elided to `$WS`/`$REPO`, `processId` to 0. |
| P10 | **Plant**: one captured record altered (a server answer in `transcripts/nvim/smoke.jsonl`) is red in CI's server lanes on three OSes; reverted. |

## 4. Result

*(scored after the captures; §3 is not edited.)*
