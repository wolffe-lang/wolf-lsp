# tl28 — the prediction, before the first change (ruling #92)

Written before any tl28 change to this repository is committed, against
trunk `5528bc1`.

**Before** (the ten completed push/pull_request runs of `ci` up to
39190513101): nineteen jobs a run, each a minute or two; **14.1
job-minutes** a run (median 14.5) and **3.0 minutes of wall** (median; max
101.1, queueing). The cost here is slots: nineteen jobs at once is nearly
the org's twenty, six of them macOS (the org has five macOS slots). The old
group (`github.workflow`-`github.ref`, cancel-in-progress for every event)
also cancelled a trunk push's run when the next one landed.

**After, predicted:**

1. A harness, transcript or CI pull-request push is `tier pr`: nine jobs on
   ubuntu plus the plan and the verdict, eleven where there were nineteen,
   **no macOS or windows job, about 8 job-minutes, 3 to 4 of wall**.
2. One editor's client is `tier pr` with the server-free gate, that
   editor's lane and independence: **five jobs, about 4 job-minutes**.
   tl27's kind of change (compat files across the clients, transcripts)
   names every job, on ubuntu.
3. The `full-matrix` label at the head is `tier full`: all nineteen jobs
   plus the plan and the verdict, **about 15 job-minutes**, as before.
4. A dispatch with `-f tier=pr` of a branch that adds a path no rule names
   comes out `tier full`.
5. The planted break (an unmapped path classified pr in `ci/tiers.py`) reds
   the plan job at "the planner can say no", skips every job, and the
   verdict job, `tier unplanned`, is red: **1 to 2 job-minutes**.
