# The green-dark gates (wolf-lsp#28, lane tl10)

Two CI gates in this repository are alleged to go green with their subject
absent. This file is the lane's record: the prediction written before either
workflow was opened, then the verbatim workflow lines, then the run ids — the
green that ran nothing, the planted red, and the green at the head.

## 3. Prediction, committed before the work

Written 2026-09-21, **before `.github/workflows/ci.yml` or `nightly.yml` was
read at this head**. Inputs already re-derived at this point: `origin/trunk`
`73abae4`, `vendor/upstream/PIN` `commit = 2e4ca769…` / `version = "wolf
0.2.15 (wolfgang, pin 2e4ca76)"`, wolf-lang's latest release `v0.2.15`
(published 2026-09-17T11:09:57Z), and the run lists of both workflows.

### (a) `ci.yml` acquires wolf from the LATEST release, not the pin's tag

**Predicted: TRUE.** The orchestrator's grep of `ci.yml` for `releases/latest`
found only neovim's download because that is not how the wolf acquisition is
spelled: wolf-lsp#28 quotes a `gh release download --repo
wolffe-lang/wolf-lang --pattern "$asset"` with no `--tag`, and `gh` without
`--tag` resolves the latest release. A grep for the wrong string cannot see
the defect (wave-45's seventh false-signal shape); the orchestrator's null
result is evidence about the grep, not about the gate.

**Predicted most recent green that ran nothing: run `34682750158`** (push,
2026-09-12T08:14:14Z, sha `de053b32`, "xtask: rustfmt, and renumber
nvim-check's doc list for clippy"). That is the last wolf-lsp run made inside
the only window in the record where the pin trailed the latest release —
wolf-lang published `v0.2.13` at 2026-09-12T07:49Z while this repo pinned
0.2.12 — and it is a *push* run, so its green is the one that protects trunk.
The PR run in the same window, `34681967615` (07:55:41Z, same sha), should be
dark the same way. Every run before 07:49Z that day and every run since
2026-09-14 should have found its asset, because pin and latest have agreed
since.

**Falsifier:** if `SERVER UNAVAILABLE` appears in neither run's log, or if the
acquisition carries a `--tag`, (a) is false and I say so.

### (b) `nightly.yml` never downloads a server; `fuzz-sweep` and `latency` have never run

**Predicted: TRUE.** Nine of the last ten scheduled nightlies finish in 41–48
seconds and one in 1m18s. A fifteen-minute fuzz sweep plus a latency-budget
job cannot fit in 44 seconds; the duration alone is the tell (wave-45: "a fast
green on a slow job is a defect report"). All ten are SUCCESS.

**Predicted most recent green that ran nothing: run `35587411389`** (schedule,
2026-09-21T10:11:30Z, 44s, trunk) — today's, and every one of the ten listed
behind it, back to `34684214053` (2026-09-12).

**Falsifier:** a `release download` in `nightly.yml`, or a `Sweep` step with a
non-skipped conclusion in any of those ten runs.

### What I expect to have to build

For each claim that holds: a planted break with a cited red run, then a fix in
the tl08 loud-skip shape. For (a) the break needs a pin the latest release
cannot satisfy — pin and latest agree today, so the window is shut and must be
forced open with a **temporary** pin version reverted inside this same PR (the
contract forbids a real bump; that is ww32/bs51's wave and r21's release). For
(b) the break is the server step forced absent on a workflow that is supposed
to red when its subject is missing.

**Predicted cost of the fix to green-CI runtime:** ci.yml unchanged in the
agreeing case (the `--tag` resolves the same archive it resolves today);
nightly.yml grows a download plus a real sweep, so the nightly's 44 s becomes
minutes, and a 44-second nightly after this lands is itself a defect report.
