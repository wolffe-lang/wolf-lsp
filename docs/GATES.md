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

## 2. Inputs, verified — both claims HOLD

Re-derived against origin 2026-09-21, after the prediction commit.

### Drift in the contract's inputs line

- **`wolf-lsp` trunk is `73abae4` (tl09): CONFIRMED** against
  `origin/trunk` after `git fetch --prune`.
- **`vendor/upstream/PIN` is at 0.2.15, not 0.2.14.** The contract is right
  that PIN keys the repo to a wolf-lang commit; the shared checkout at
  `~/GithubOrgs/wolffe-lang/wolf-lsp` is **three commits behind origin, sitting
  on local `trunk` at `498d9cc` (tl07's changelog)** with a dirty `upstream`
  gitlink, so reading PIN out of that working tree answers
  `commit = 30731a6…` / `wolf 0.2.14`. At `origin/trunk` it is
  `commit = 2e4ca769b396219585a07ff18492529c944672d9` and
  `version = "wolf 0.2.15 (wolfgang, pin 2e4ca76)"`. This lane worked from
  `origin/trunk` in a worktree. Reported, not absorbed.
- **wolf-lang's latest release is `v0.2.15`** (published 2026-09-17T11:09:57Z,
  four assets), which **equals the pin**. So claim (a)'s window is shut
  *today*: that is why the gate is currently green for the right reason, and
  why the break had to be planted to be seen.

### (a) HOLDS — `ci.yml` acquires from the LATEST release

`.github/workflows/ci.yml` lines 462–471 at `73abae4`, verbatim:

```yaml
          n_rel=$(gh release list --repo wolffe-lang/wolf-lang --limit 1 \
            --json tagName --jq 'length' 2>/dev/null || echo 0)
          if [ "$n_rel" -gt 0 ]; then
            # A release exists — try the asset for OUR pinned version. A
            # release at a different version than this repo's pin carries no
            # matching asset, and the lane stays dark honestly.
            if gh release download --repo wolffe-lang/wolf-lang \
                 --pattern "$asset" -D .wolf-bin 2>/dev/null; then
```

There is **no `--tag`**, so `gh release download` resolves wolf-lang's latest
release and looks for the pin's asset name inside it. The orchestrator's grep
for `releases/latest` could not see this: that string appears once in the file,
at line 172, and it is neovim's. A search that cannot reach the defect returns
NONE whether or not the defect exists (wave-45, the seventh shape).

Two aggravations the issue does not name:

1. `2>/dev/null` on the download discards `gh`'s own diagnosis, so the log
   cannot distinguish "wrong release" from "no such asset" from "network".
2. the guard above it, `gh release list --limit 1 … | length`, only asks
   **whether any release exists**. It is true for every release wolf-lang will
   ever cut, so it never protects the branch below it.

**The green that ran nothing: run `34682750158`** (push to trunk,
2026-09-12T08:14:14Z, sha `de053b32`), every job `success`, and in the
`server lane (ubuntu-latest)` log:

```
2026-09-12T08:14:29.3748766Z SERVER UNAVAILABLE: releases exist but none carries wolf-0.2.12-x86_64-unknown-linux-gnu.tar.gz
2026-09-12T08:14:38.0572923Z   verdict    SERVER UNAVAILABLE — no wolf binary at pin a7f517e (…)
```

The PR run at the same sha, `34681967615` (07:55:41Z), is dark identically.
wolf-lang had published `v0.2.13` at 2026-09-12T07:49:11Z; `v0.2.12`'s own tag
carried all four assets the whole time
(`wolf-0.2.12-{aarch64-apple-darwin,aarch64-unknown-linux-gnu,x86_64-pc-windows-msvc,x86_64-unknown-linux-gnu}.tar.gz`,
published 2026-09-12T00:47:09Z). So the archive the pin needed existed, was
reachable by tag, and the workflow asked the wrong release for it — for
**twenty-five minutes either side of a merge**, on all three OSes.

### (b) HOLDS — `nightly.yml` never downloads a server

`grep -c 'release download' .github/workflows/nightly.yml` = **0** at
`73abae4`. The `server availability` job in full (lines 36–66) installs a
toolchain and runs `doctor`; there is no acquisition anywhere in the file:

```yaml
  server:
    name: server availability
    runs-on: ubuntu-latest
    outputs:
      available: ${{ steps.doctor.outputs.available }}
      pin: ${{ steps.doctor.outputs.pin }}
    steps:
      - uses: actions/checkout@v5
      - name: Install the pinned toolchain
        run: rustup show active-toolchain
      - name: lspconf doctor
```

`doctor` on a runner with no binary exits 77, so `available=no` on every
nightly that has ever run, and `Sweep`, `Measure` and `Keep the numbers` are
all `if: needs.server.outputs.available == 'yes'`.

**The green that ran nothing: run `35587411389`** (schedule,
2026-09-21T10:11:30Z, trunk, 44 s). Job conclusions: all five `success`.
Step conclusions in it:

```
fuzzed partial-edit sweep (15 min)   SKIP — no server at the pin   success
fuzzed partial-edit sweep (15 min)   Sweep                         skipped
latency budgets (D5 JSONL…)          SKIP — no server at the pin   success
latency budgets (D5 JSONL…)          Measure                       skipped
latency budgets (D5 JSONL…)          Keep the numbers              skipped
```

The ten most recent scheduled nightlies (`35587411389`, `35502306171`,
`35433390511`, `35328167290`, `35206196254`, `35079704971`, `34953280083`,
`34831470285`, `34750053908`, `34684214053`) are all `success` and all
41 s – 1 m 18 s. A job named "15 min" that finishes in 44 seconds is the
duration tell on its own.

### A third dark gate, reported not fixed (out of #28's scope)

`nightly.yml`'s `drift` job (lines 169–212) downloads nothing either. Its
`Attempt the latest wolf-lang artifact` step only counts releases
(`n_rel > 0 → server=maybe`) and the `Report` step prints prose about what
somebody could do with an artifact. It is report-only by design, so it cannot
be silently green about a test it did not run — but "capability drift vs
wolf-lang HEAD" has never compared anything to wolf-lang HEAD. Filed here for
whoever takes it; this lane does not touch it.

## 4. Evidence index

Every run below is `wolffe-lang/wolf-lsp`. "Dark" means the server-dependent
steps did not execute; "green" is the run's own conclusion.

| # | run | sha | what it proves |
|---|---|---|---|
| 1 | `34682750158` | `de053b32` (trunk) | **(a) green that ran nothing.** Push to trunk 2026-09-12, every job `success`, `SERVER UNAVAILABLE: releases exist but none carries wolf-0.2.12-x86_64-unknown-linux-gnu.tar.gz`, server lane dark on all three OSes. |
| 2 | `35587411389` | trunk | **(b) green that ran nothing.** Scheduled nightly 2026-09-21, 44 s, all five jobs `success`, `Sweep`/`Measure`/`Keep the numbers` all `skipped`. |
| 3 | `35669433451` | `196b53e` | **(a) reproduced at this head, live.** The plant only — PIN untouched, workflow unchanged — and the run is **green** with the server lane dark on ubuntu, macOS and windows: `SERVER UNAVAILABLE: releases exist but none carries wolf-0.2.99-<triple>.tar.gz`. The 2026-09-12 window is not history; the gate is dark today and was dark all week for want of a mismatch. |
| 4 | `35669729227` | `39c99a6` | **(a) PLANTED RED.** Same plant, fixed acquisition: `server lane` **failure** on all three OSes — `##[error]no wolf-lang release at v0.2.99 — this repo pins wolf 0.2.99 at 2e4ca769…`, with `gh: release not found` quoted from gh's own stderr. |
| 5 | `35669926832` | `64102d0` | **The one green-while-dark path, exercised.** Plant changed to `0.2.15+dev.unknown` (an unreleased trunk pin, which `PIN` explicitly permits): green, with `##[warning]server lane dark: PIN is at an UNRELEASED wolf (0.2.15+dev.unknown)` and `##[notice]server lane skipped — PIN is at an unreleased wolf`. The `doctor` guard did not misfire: the log shows it evaluating `[ "none" = "downloaded" ]`. |
| 6 | `35670143189` | `63acadf` | **The fix's own defect, caught by its own red.** Plant reverted, pin back at 0.2.15 — and the run is **red**: `gh: unknown flag: --tag`. `gh release download` takes the tag **positionally**. wolf-lsp#28's suggested fix (`--tag "v$version"`) does not exist as a flag, and under the old `2>/dev/null` this would have printed the same SERVER UNAVAILABLE sentence forever while looking exactly like a gate doing its job. Keeping stderr is what made it one run instead of one pin cycle. |
| 7 | `35670361426` | `aeb39f6` | **(a) green at the fix.** `gh release download "$tag" …`: `acquired wolf-0.2.15-x86_64-unknown-linux-gnu.tar.gz from v0.2.15`, `verdict READY — .wolf-bin/wolf serves LSP at pin 2e4ca76`, and `type-names-check`, `Conformance replay`, `One truth`, the five suites and the seeded fuzz all `success` on all three OSes. |
| 8 | `35670767375` | `472130e` | Same, through the shared `./.github/actions/acquire-wolf`: green, `Report the skip` `skipped`, the five server-dependent steps `success` ×3. |
| 9 | `35671150476` | `ae3a412` | **(b) PLANTED RED.** The nightly with its acquire step commented out — "the server step forced absent", exactly trunk's shape — and the new gate: `server availability` **failure**, `##[error]nightly has no wolf server at pin 2e4ca769…`, `fuzz-sweep` and `latency` **skipped**, run conclusion **failure**. Trunk's nightly at the same state is run 2 above, green. |
| 10 | `35671351880` | `e8afc02` | **(b) green at the fix, and the first night this repository has ever measured anything.** `server availability` acquires and `A nightly with no server is RED` is `skipped`; `latency` runs `Measure` and `Keep the numbers`, publishing artifact `lsp-latency-jsonl` (1,927 bytes) — the D5 JSONL that has been "report-only" since ls01 and has never once been reported. `fuzz-sweep` runs the 15-minute sweep for the first time. |

Committed files, for the claims that are not runs:

- `.github/actions/acquire-wolf/action.yml` — the single acquisition.
- `.github/workflows/ci.yml`, the `server-lane` job — calls it; `if:` on the
  five server-dependent steps is now `== 'downloaded'`.
- `.github/workflows/nightly.yml`, the `server`, `fuzz-sweep` and `latency`
  jobs — the acquisition the file never had, and the red.
- `vendor/upstream/PIN` — **unchanged by this PR**. `git diff origin/trunk --
  vendor/upstream/PIN` is empty at every commit on this branch. The pin bump is
  ww32/bs51's wave and r21's release, and the temporary version overrides that
  forced the defect's window open were in the workflow's own acquisition step,
  never in PIN, and are gone from the head of this branch (commits `63acadf`
  and `e8afc02` remove them).

## 5. Done-when

- [x] branch `tl10` on origin
- [x] PR #29 open, unmerged
- [ ] CI green at the head sha *(filled in at the end of the lane)*
- [x] #28's two claims each verified verbatim, each seen red before it was trusted
- [x] §2 drift reported (the shared checkout three commits behind; the pin is 0.2.15, not 0.2.14)
- [x] §3 prediction commit `69275f6` precedes every workflow read
- [x] five sections present
- [x] the pin never moved
