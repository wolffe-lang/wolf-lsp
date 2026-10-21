# tl30 — the macOS cap: the prediction, before the first change

Written before any tl30 change to this repository is committed, against
trunk `76cca08`. Contract: `sprints/tooling/tl30-footprints-and-the-macos-cap.md`
(planning repo `wolffe-lang/wolf`, trunk `809e40f`), under `sprints/RULES.md`.
The org-wide census (every repository's macOS jobs, the last 60 runs each
ran one, every red macOS job against its Linux counterpart) is wolf-lang's
`ci/tl30.md`; this is wolf-lsp's slice.

**Why.** Since the Team plan (2026-10-20) the Linux pool is 60 slots and the
macOS pool is still 5: tl29 measured all five macOS slots busy 55 % of the
time after the switch, a median macOS wait of 17.6 minutes; pax's
half-minute `macos-run` waited 44.6. What binds is the slot, not the
minutes: wolf-lsp's merge gate (`tier full`: the `full-matrix` label, and
again on the trunk push) asks for **five** macOS slots, `test`, `nvim
plugin`, `server lane`, `helix config` and `emacs mode` on macOS, 4.3 to
4.5 macOS-minutes between them, each waiting for its own slot.

**What the macOS jobs caught.** The last 60 runs with a macOS job
(35271179423 .. 39439988810, 2026-09-17 to 10-20): 10 red macOS jobs, and
in every one the same job on ubuntu was red in the same attempt (tl10 to
tl27's planted breaks and shared reds). **No macOS job caught a red that
no Linux job caught.** So the merge keeps one macOS smoke and the rest goes
to the nightly.

**The smoke** is `test (macos-latest)`: the pinned toolchain for
aarch64-apple-darwin, rustfmt, clippy, the workspace's tests (the server
built and exercised on macOS) and the pin's sync, about one minute. The
editor clients on macOS (nvim, the server lane, helix, emacs) leave the
merge for the nightly.

## The shape

| tier | when | test | nvim, server lane, helix, emacs | vscode, the rest |
|---|---|---|---|---|
| `all` (new) | the nightly; `workflow_dispatch -f tier=all` | ubuntu, macOS, windows | ubuntu, macOS, windows | as before |
| `full` | the `full-matrix` label (the merge gate); a trunk push; a dispatch (the default); a path no rule names | ubuntu, macOS, windows | ubuntu, windows | as before |
| `pr` | any other pull-request run; `-f tier=pr` | ubuntu | ubuntu | ubuntu |
| `noop` | prose alone; another label | — | — | — |

The nightly is `tier all`, every job on every host as `tier full` was
before tl30, and its red still opens the `nightly-red` issue
(`ci/nightly-red.sh`, unchanged). `lane-audit.sh` accepts a green `tier
all` as the superset of `tier full` (tl29); the merge rule is unchanged.

## Prediction

Before (the labelled and trunk-push `tier full` runs 39361548798 and
39368493479, and the 54 untiered push and pull-request runs before tl28,
which ran the same five macOS jobs): **4.3 to 4.5 macOS-minutes and five
macOS slots a run** (test 0.9, server lane 2.3, nvim 0.5, helix 0.4, emacs
0.3), 14.5 job-minutes.

1. `tier full` (the label and every trunk push): **about 1 macOS-minute and
   one macOS slot a run** (falsified above 2 or above one slot), about 11
   job-minutes (falsified above 14). About **−3.5 macOS-minutes and four
   macOS slot waits a run, −7 and eight a merge** (the label's run and the
   trunk push's).
2. `tier all` (the nightly, `-f tier=all`): unchanged, every job on every
   host.
3. `tier pr`: unchanged (ubuntu alone).
4. The planted break (`full_leg` answering every leg, the merge's shape
   before tl30) reds `ci/tiers-test.py` in the plan job of a dispatch
   `-f tier=pr` on its own branch; nothing else runs; under 2 job-minutes.
5. This pull request touches `ci/` and `.github/`, so its pull-request run
   is `tier pr`; the labelled run at the final head is the first `tier
   full` of the new shape and is the measurement (the orchestrator watches
   the run, RULES §6).
