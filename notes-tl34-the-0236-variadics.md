# tl34 — the pin at 0.2.36: the variadics and the argv readers (wolf-lsp)

The lane's note. The full record (§1 forbidden, §2 inputs, §3 prediction and
its scoring, §4 evidence index) is `docs/PIN-0236.md`; the grammar's half is
tree-sitter-wolf `docs/spec-findings-tl34.md`. Contract: planning
`sprints/lsp/tl34-the-0236-variadics.md`.

## Forbidden
- No merge, label, tag, release, issue action or attribution trailer; no wait
  on CI. kasumi never touched. No package installed.
- Every cargo, lspconf and gate run on hasu, CPU block B, holding both lock
  names, from a worktree pinned to a commit; the Mac for git, `gh`, the
  darwin probes and the xtask generators, behind the Mac build lock.
- No change to the server, the harness's matching or the normalizer to make a
  transcript pass; no client gains a list of prelude functions (ruling #77).
- The pin is the release by digest, each read twice against
  `sprints/release/r41-evidence.md`; never wolf-lang's trunk.

## Inputs, verified
- wolf `v0.2.36` = `5adf3d91`, release 427033151; lupin `v0.1.58` =
  `6240efe5`; the bundled std wolf-std `78b9d3cb`. The linux x86-64 and macOS
  arm64 archives of each downloaded and hashed twice, equal to the API and to
  r41-evidence.
- `spec/prelude.json` 207 → 214 names, as `(name, kind)` sets both ways:
  `VaList` (type), `va_start`, `va_arg`, `va_copy`, `va_end`, `env_arg0`,
  `env_args_bytes` (functions); nothing else; marks unchanged.
- `spec/grammar.ebnf` gains `'...'`; `spec/anchors.json` +13 keys; the
  server's crates (`wolf_query`, `wolf_lsp`) are the same trees at both tags;
  the std's `list/list.lu` moved.
- `VaList` is E0826 on macOS (and by the clause on windows) at the type's
  name itself; clean on linux x86-64.

## Prediction
Committed at `59d6a33` before the pin moved; eleven rows, nine held, two are
CI's. In short: the prelude row red at the 0.2.35 vendor (`4eb964b`); at the
bump, exactly `the_vendored_list_at_the_pin_is_exactly_ours` and
`type-names-check` red on `VaList`, `grammar-drift` red on `...` and the pin,
`nvim-check` on `pin.lua`, `compat-check` on the pins (`2a6f475`); 72 of 73
re-recorded transcripts header-only and `std/reach`'s definition 576 → 565;
the six smokes matched with only the header swapped; `requests/hover-argv`
green at 0.2.36 and red against the 0.2.35 server.

R-list (each implemented as recommended):
- **R-tl34-1** `VaList` in the regular artifacts: (A) `TYPE_POSITION_UNPAINTED`
  with its reason: a prelude `type`, not one of the scalars `TYPE_NAMES`
  mirrors, and E0826 off x86-64 Linux at the name (implemented, `0268161`);
  (B) `TYPE_NAMES`, painted by the TextMate grammar, vim and emacs as a
  builtin type.
- **R-tl34-2** the variadics on the wire: (A) measured on hasu's linux server
  and recorded in `docs/PIN-0236.md`, no transcript, because the replay runs
  on three OSes and E0826 changes the diagnostics on two (implemented); (B)
  teach the script DSL an OS gate (a harness change, a later lane); (C) a
  `#[cfg]`-gated document, whose answers still differ by OS.
- **R-tl34-7** the EBNF's `'...'` in the TextMate grammar: (A) the derived
  default, an operator, matched longest first ahead of `..=` and `..`
  (implemented, `0dd2b75`); (B) list it in `DELIMITERS`, which leaves `...`
  painted as the operator `..` and a stray `.`.

## Evidence index
- `59d6a33` docs: predict the 0.2.36 pin
- `4eb964b` xtask: the 0.2.36 prelude names, red at the 0.2.35 vendor; the unreleased-name half retires
- `2a6f475` upstream: pin bumped to 5adf3d9 (v0.2.36); re-vendored
- `0268161` xtask: VaList is a prelude type the regular artifacts leave unpainted (R-tl34-1)
- `15c6f5e` vendor: README type count at v0.2.36
- `c8ccd83` vendor: os/arg0.lu joins the samples, s261's argv readers at v0.2.36
- `25c0e36` transcripts: requests/hover-argv, the script for 0.2.36's argv readers
- `0dd2b75` vscode: grammar regenerated; the EBNF's '...' joins the operator inventory
- `b412e45` clients: pin.ts and pin.lua regenerated at 5adf3d9
- `a6a5d8b` transcripts: re-record at 0.2.36 (72 scripted, header-only)
- `3e86caa` transcripts: std/reach's definition follows wolf-std 78b9d3c's list.lu (line 576 to 565)
- `459cdac` transcripts: the six smokes replay at 5adf3d9, restamped
- `47f5ec9` transcripts: requests/hover-argv recorded at 0.2.36
- `3397bc1` compat: the statement moves to v0.2.36
- `30c69e6` docs: matrix stamp at 5adf3d9; 80 replay, the six smokes restamped
- `e864a3d` docs: tl34 scored, the variadics measured
- `10c1175` changelog: tl34
- `4e190ba` compat: regenerated
- this note

Logs: hasu `/home/mfwolffe/lanes/tl34/g0`–`g3` and the final gauntlet's
directory (the PR body); nomad `~/lanes/tl34/` (`acquire/`, `probes/`,
`g-gen1/`, `g-gen2/`, `g3h/`).

## Done-when
- [x] the pin at 0.2.36, the vendored data the tag's blobs
- [x] the seven names vendored, `VaList` classified, rows for each red first
- [x] completion, hover and signature rows for the argv readers on the wire
  (`requests/hover-argv`); the variadics' measured in the record
- [x] the six smokes replayed, restamped, none re-captured (none moved)
- [x] README read, current
- [ ] CI `tier pr` green at the head; `tier full` at the labelled head

Issue to file: move helix `rev` and zed `commit` (`936a96f5`) to
tree-sitter-wolf's tl34 merged sha.
