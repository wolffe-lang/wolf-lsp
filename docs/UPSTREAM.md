# Upstream integration status

A track whose value is integration must report integration **status**, not
intent (ls07 §5). This file is that report, and its rule is that a row states a
*state* rather than a plan.

**The facsimile series is MERGED; everything else is unsubmitted.** For one
sprint this file said nothing had shipped anywhere while facsimile's whole
wolf series sat merged in that repo's own trunk (`21e58aa`, 2026-08-10, on
its origin) — a ledger saying work is unsubmitted when it is merged is worse
than no ledger, and le01's reconciliation (2026-08-27) corrected six rows.
Everything else below remains genuinely unsubmitted. The old blanket reason
has also expired: `wolf-lang` now tags releases (v0.1.0 "wolfgang",
2026-08-12, published with artifacts), so "a language nobody can install" no
longer gates the registry rows — what gates each row now is written on the
row.

## The state vocabulary

Exactly five words. `cargo xtask release-check` step 9 fails if a row uses none
of them, because a row that says something else is a promise rather than a
state.

| state | means |
|---|---|
| `NOT SUBMITTED` | written and reviewable here; no PR opened |
| `SUBMITTED` | a PR exists and is open. **Not the same row as merged** |
| `MERGED` | landed upstream |
| `DECLINED` | upstream said no. The reason is recorded, and it stays in the table |
| `ABANDONED` | we stopped pursuing it. The reason is recorded, and it stays in the table |

A row is never deleted. A table that forgets its declines is a table that
proposes the same patch twice.

## fackr — `wolffe-lang/fackr`, read at `496c7e2` (v1.2.1)

Series: [`clients/fackr/patches/`](../clients/fackr/patches/) ·
decomposition and per-PR file lists:
[`STATUS.md`](../clients/fackr/patches/STATUS.md)

| PR | change | state | note |
|---|---|---|---|
| PR1 registration | `.lu`/`.wolfi` → languageId `wolf`, `ServerConfig` row | `NOT SUBMITTED` | committed locally as `e9ed924` on `wolf-lsp-integration` (whole series, one commit); unpushed by working agreement. le01 re-based it onto trunk as `le01-wolf-land` — a fast-forward, trunk had not moved — with the suite green |
| PR2 syntax | `Language::Wolf`, `wolf_def()` from the pinned grammar | `NOT SUBMITTED` | generated; regenerate before submitting if the pin moved |
| PR3 installer panel | `KnownServer` row routing to the manual-info dialog | `NOT SUBMITTED` | |
| PR4a framing | `FrameBuffer` — frame on bytes, decode whole messages | `NOT SUBMITTED` | the highest-value patch in the series; fixes a real split-multibyte bug |
| PR4b stderr | bounded drain thread, 64-line tail | `NOT SUBMITTED` | fixes a ~64 KiB deadlock |
| PR4c position encoding | declare `["utf-32"]`, read and honour the reply | `NOT SUBMITTED` | |
| PR4d server messages | `window/logMessage` reaches a bounded client log | `NOT SUBMITTED` | |
| PR-test live smoke | two `#[test]`s against a real `wolf lsp`, skipping loudly | `NOT SUBMITTED` | needs an installable `wolf` to be useful upstream |
| PR5 CI lane | a minimal build+test workflow | `NOT SUBMITTED` | **offered, not written** — `clients/fackr/README.md` §"The CI lane fackr does not have" |
| PR-compat version check | compare `wolf --version` against a declared range, once, into fackr's log surface | `NOT SUBMITTED` | **offered, not written.** The series predates `COMPAT.md`; the range statement travels with the patch, under fackr's version scheme, so this is a follow-up PR rather than a rider |

## facsimile — `FortranGoingOnForty/facsimile`, series MERGED at `21e58aa` (2026-08-10)

Series: [`clients/facsimile/patches/`](../clients/facsimile/patches/) ·
[`STATUS.md`](../clients/facsimile/patches/STATUS.md). Written against
`1242ffa` (v0.32.8); the six written patches landed **as one commit**,
`21e58aa`, on that repo's trunk (and its origin) the same day — this table
said `NOT SUBMITTED` for all six until le01's reconciliation, 2026-08-27.
Each row below was re-verified against facsimile trunk source, not the
commit message.

| PR | change | state | note |
|---|---|---|---|
| PR1 registration | languageId `wolf`, `add_config` with only the four flags wolf serves | `MERGED` | in `21e58aa` |
| PR2 installer entry | count 20 → 21 **and the missing `i = i + 1`** | `MERGED` | in `21e58aa`; fixes an out-of-bounds write that predates wolf |
| PR3 syntax + comments | `load_wolf_syntax`, `//` comments, `"""` before `"` | `MERGED` | in `21e58aa`. le01 then found the state's exit bug it shipped with (below) |
| PR4a position encoding | declare `["utf-16"]` | `MERGED` | in `21e58aa`; correct-by-declaration, not by default |
| PR4b formatting keybinding | `case('alt-shift-f', 'shift-alt-f')` | `MERGED` | in `21e58aa`; formatting was uninvokable before this |
| PR4c hot-path hygiene | drop an unconditional `/tmp` debug append from `didChange` | `MERGED` | in `21e58aa` (the removal comment in `lsp_server_manager_module.f90` records it) |
| PR5 server-request replies | answer `workspace/configuration` etc. | `NOT SUBMITTED` | **offered, not written** — `handle_request` is still a TODO stub in trunk. wolf needs none of it; any other server does |
| PR-test live smoke | — | `NOT SUBMITTED` | **offered, not written** |
| PR-compat version check | the same one-shot comparison, into facsimile's log surface | `NOT SUBMITTED` | **offered, not written**, same reasoning as fackr's |
| PR6 multiline-string exit fix | trim the blank-padded stored `"""` so multiline string mode exits; regression test; python triple-quote ordering | `NOT SUBMITTED` | found and fixed by le01 on facsimile branch `le01-wolf-multiline-string` (2 commits, suite green). The merged PR3 table shipped with this bug: every line after the first `"""` block rendered as string |

The fackr series (and facsimile's three unwritten offers) carry a **standing
re-verification obligation**: each was written against one upstream commit,
and neither upstream is pinned by anything in this repository. A patch series
against a moved `trunk` is a merge conflict somebody discovers during review.
Re-apply and re-run the gates in `STATUS.md` before opening anything. (le01
re-verified fackr's: trunk is still `496c7e2`, so the series applies as a
fast-forward today.)

## Registries and downstream entries

| target | what would be submitted | state | gating |
|---|---|---|---|
| [`neovim/nvim-lspconfig`](https://github.com/neovim/nvim-lspconfig) | `lsp/wolf.lua` **verbatim** — since lspconfig 2.0 an entry *is* an `lsp/<name>.lua` returning a config table (`clients/nvim/README.md`) | `NOT SUBMITTED` | the s66 gate CLEARED 2026-08-12 (wolf-lang v0.1.0 is published and installable); what remains is the human act of opening the PR, per `RELEASE.md` |
| [`mason-org/mason-registry`](https://github.com/mason-org/mason-registry) | a package definition pointing at wolf-lang's **release artifacts** | `NOT SUBMITTED` | the s66 gate CLEARED 2026-08-12 — v0.1.0 ships artifacts to point at. Note D34 still: mason would install the *compiler*, which is wolf-lang's publisher decision to make, not this repo's |
| [`zed-industries/extensions`](https://github.com/zed-industries/extensions) | the `zed_wolf` extension as a registry submodule entry | `NOT SUBMITTED` | s66 cleared, but `docs/MATRIX.md`'s Zed row still gates: submitting an extension nobody has ever run in Zed is the fabrication that file exists to prevent |
| [`wolffe-lang/tree-sitter-wolf`](https://github.com/wolffe-lang/tree-sitter-wolf) | a real `grammar.js` — the repo is a seed commit (`b1b2c17`) with none | `NOT SUBMITTED` | nobody's sprint. **This is the largest standing gap in the track**: it is why `.lu` buffers in Helix and Zed have no highlighting at all, and why both editors ship their grammar block commented out |
| Open VSX namespace `wolf-lang` | `ovsx create-namespace` + `ovsx publish` | `NOT SUBMITTED` | needs an Eclipse Foundation token — [`DISTRIBUTION.md`](DISTRIBUTION.md) §OWED TO HUMAN |
| VS Marketplace publisher | the publisher identity the vsix's `publisher` field names | `NOT SUBMITTED` | `wolf-lang-unpublished` is a deliberate placeholder. Registration is a human act with a credential this repo must never hold |

## What refreshes this file, and when

`cargo xtask release-check` verifies that every row uses the vocabulary. It
**cannot** verify that a row is true — nothing in this repository can observe a
PR moving from open to merged. So step 9b of the checklist is permanently
`PENDING`: before tagging, open each link and re-read the state. A `SUBMITTED`
row that quietly became `MERGED` is exactly the drift this table exists to
catch, and the only mechanism that catches it is a person.
