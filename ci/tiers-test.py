#!/usr/bin/env python3
"""ci/tiers-test.py — the planner's own proof (tl28; wolf-lang's
ci/tiers-test.py, tl21, cut to wolf-lsp). python3, git and bash only.

    ci/tiers-test.py

  T1  the tiers by event: a trunk push, the nightly, a dispatch (tier full,
      or no tier) and the `full-matrix` label are full on every host;
      another label's `labeled` event is a noop; a dispatch's tier must be
      full or pr.
  T2  the pull-request tiers by path: each path names the jobs THIS
      repository's block below says, on the pull request's hosts; prose
      alone is noop; prose beside code is that code's tier.
  T3  full wins: a path no rule names, alone or beside anything; changed
      paths that cannot be read; a change that names no path; a dispatched
      branch the same.
  T4  the wiring of .github/workflows/ci.yml: the plan is first and the
      verdict last but the nightly's issue; every job needs the plan, runs
      only when the plan names it and takes its hosts from the plan; every
      plan output is read with a default; the `tier` job needs every job,
      runs on `!cancelled()` and checks with `ci/tiers.py verdict`;
      cancel-in-progress is for pull requests only and every other run has a
      concurrency group of its own run id; the trunk push (and no other
      branch's), the nightly, the label and the dispatch's `tier` input are
      there; the full tier's hosts are the ones the jobs ran on before tl28.
  T5  the tripwire: no line of this repository's code that reads a file
      names a path the planner calls prose (THIS repository's block). The
      search is shown to fire on paths it must find. (A WARN, not a
      failure: a tracked path no rule names; its pull request runs full.)
  T6  the verdict: green for a full, a pr and a noop run; red for a chosen
      job failed, skipped or cancelled, a job that ran unasked, a failed
      plan, a plan with no tier, a full run that chose less, a full run on
      the pull request's hosts, and green when GitHub drops an output that
      would have been empty.
  T7  the change read from git: a real merge commit against its first
      parent, with the base moved on meanwhile; red-to-full when HEAD is no
      merge or its second parent is not the pull request's head; a
      dispatched branch against its merge base with origin/trunk, and full
      when there is no origin/trunk to read.
  T8  ci/nightly-red.sh against a stand-in `gh`: no open issue opens one,
      labelled, under the one title; an open one gets a comment and no new
      issue; a label that already exists is no failure.

Exit 0 when every check passes, 1 otherwise.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PLAN = os.path.join(ROOT, "ci", "tiers.py")
RED = os.path.join(ROOT, "ci", "nightly-red.sh")
ENV_KEYS = ("EVENT", "ACTION", "LABEL", "LABELS", "HEAD_SHA", "IN_TIER", "NEEDS")
fails = 0


def say(ok, what):
    global fails
    print("tiers-test: %s %s" % ("PASS" if ok else "FAIL", what))
    fails += not ok


# --- this repository's -------------------------------------------------------------
NIGHTLY = None            # (its tier, its jobs); None: the full tier
FULL_LEG_TIERS = ("full",)  # the tiers that run every leg
WORKFLOW_FILE = "ci.yml"
WORKFLOW_NAME = "ci"
EVERY_JOB = ["test", "nvim-plugin", "vscode-extension", "vscode-package", "server-lane", "helix-config",
             "zed-extension", "emacs-mode", "independence"]
WORKFLOW_JOBS = ["plan"] + EVERY_JOB + ["tier", "nightly-red"]
FULL_JOBS = EVERY_JOB
PUSH_BRANCHES = "trunk"
OTHER_HOST_JOBS = ()
# The hosts before tl28 (.github/workflows/ci.yml at trunk 5528bc1).
_H = [{"os": "ubuntu-latest"}, {"os": "macos-latest"}, {"os": "windows-latest"}]
BEFORE_HOSTS = {"matrix_test": _H, "matrix_nvim_plugin": _H, "matrix_vscode_extension": _H[:1],
                "matrix_server_lane": _H, "matrix_helix_config": _H, "matrix_emacs_mode": _H}
ALL_HOSTS = BEFORE_HOSTS
PR_HOSTS = {k: _H[:1] for k in BEFORE_HOSTS}
CODE_PATH = ("crates/lsp_harness/src/replay.rs", EVERY_JOB)
NVIM = ["test", "nvim-plugin", "independence"]
VSCODE = ["test", "vscode-extension", "vscode-package", "independence"]
CHECKS = ["test", "independence"]
PR_PATHS = [(p, EVERY_JOB) for p in [
    "src/main.rs", "crates/lsp_harness/src/replay.rs", "xtask/src/config.rs", "tests/nvim_plugin.rs",
    "fixtures/regions.lu", "transcripts/annotate/inlayHint-nvim.jsonl", "profiles/fackr.json", "vendor/upstream/PIN",
    "upstream", "assets/logo.svg", "divergences.toml", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml",
    "clippy.toml", "deny.toml", ".cargo/config.toml", ".github/workflows/ci.yml", ".github/actions/acquire-wolf/action.yml",
    "ci/tiers.py", "ci/tiers-test.py", "ci/nightly-red.sh", "LICENSE", ".gitmodules", ".gitignore", ".gitattributes",
    ".editorconfig"]] + [
    ("clients/nvim/lua/wolf/compat.lua", NVIM), ("clients/nvim/compat.json", NVIM),
    ("clients/vscode/src/compat.ts", VSCODE), ("clients/vscode/package.json", VSCODE),
    ("clients/helix/languages.toml", ["test", "helix-config", "independence"]),
    ("clients/zed/extension.toml", ["test", "zed-extension", "independence"]),
    ("clients/emacs/wolf-mode.el", ["test", "emacs-mode", "independence"]),
    ("clients/fackr/README.md", CHECKS), ("clients/yew/init.fl", CHECKS), ("clients/README.md", CHECKS),
    ("docs/COMPAT.md", CHECKS), ("docs/MATRIX.md", CHECKS), ("README.md", CHECKS), ("CHANGELOG.md", CHECKS)]
PROSE_PATHS = ["CONTRIBUTING.md"]
PR_UNIONS = [(["clients/nvim/compat.json", "clients/vscode/src/compat.ts"], ["test", "nvim-plugin", "vscode-extension", "vscode-package", "independence"]),
             (["clients/zed/extension.toml", "docs/COMPAT.md"], ["test", "zed-extension", "independence"])]
READS = re.compile(r"read_to_string|read_dir|File::open|fs::read\b|\.join\(|Path::new|PathBuf::from|is_file|is_dir|include_str!|"
                   r"readFile|readdir|existsSync|\bopen\(|path\.join|path\.resolve|io\.open|vim\.fn\.readfile|insert-file-contents")


def is_code(f):
    """The code that could read a prose path: every Rust file, and the
    clients' own code (TypeScript, Lua, Emacs Lisp)."""
    return f.endswith((".rs", ".ts", ".js", ".mjs", ".lua", ".el"))


MUST_FIND = ["clients/helix/languages.toml", "CHANGELOG.md", "profiles/fackr.json"]
RULED = {}  # {prose root: {regex of a line that names something else: why}}


def extra_t5(files, cls, say):
    every = " ".join(EVERY_JOB)
    core = [p for p in files if p.startswith(("src/", "crates/", "xtask/", "tests/", "transcripts/", "profiles/"))]
    say(len(core) >= 150 and all(cls.get(p) == every for p in core),
        "T5 every tracked path under src/, crates/, xtask/, tests/, transcripts/ and profiles/ names every job (%d paths)" % len(core))


def extra_checks(say, plan, expect, blocks, wf):
    pr = dict(event="pull_request", action="synchronize")
    clients = [p for p, _ in PR_PATHS if p.startswith(("clients/", "docs/"))]
    say(not [p for p in clients if "server-lane" in plan([p], **pr)["jobs"]], "T2 no client or docs path runs the server lane")
    say(all("test" in plan([p], **pr)["jobs"] and "independence" in plan([p], **pr)["jobs"] for p, _ in PR_PATHS),
        "T2 every pull-request path runs the server-free gate and independence")
    say("    # ubuntu only: the darwin-arm64 @vscode/test-electron download spawns\n" in blocks["vscode-extension"],
        "T4 the VS Code lane keeps its reason for ubuntu alone")
    say("github.ref }}" not in wf, "T4 the old per-ref group that cancelled trunk's runs is gone")
# --- end of this repository's ------------------------------------------------------
WORKFLOW = os.path.join(ROOT, ".github", "workflows", WORKFLOW_FILE)
NIGHTLY = NIGHTLY or ("full", FULL_JOBS)


def clean_env(**more):
    env = {k: v for k, v in os.environ.items() if k not in ENV_KEYS}
    env.update(more)
    return env


def plan(paths=None, **kw):
    args = [sys.executable, PLAN, "plan", "--json"]
    flags = kw.pop("flags", [])
    for k, v in kw.items():
        args += ["--" + k.replace("_", "-"), v]
    args += flags
    if paths is not None:
        args += ["--paths-from", "-"]
    r = subprocess.run(args, input="\n".join(paths or []) + "\n", capture_output=True, text=True, env=clean_env())
    if r.returncode != 0:
        return {"tier": "ERROR rc=%d %s" % (r.returncode, r.stderr.strip()[-200:]), "jobs": [], "matrix": {}}
    return json.loads(r.stdout)


wf = open(WORKFLOW, encoding="utf-8").read()
JOBS = re.findall(r"^  ([A-Za-z0-9_-]+):\s*$", wf[wf.index("\njobs:\n"):], re.M)
EVERY = [j for j in JOBS if j not in ("plan", "tier", "nightly-red")]


def expect(name, got, tier, jobs):
    why = []
    if got["tier"] != tier:
        why.append("tier %s, wanted %s" % (got["tier"], tier))
    if got["jobs"] != jobs:
        why.append("jobs %s, wanted %s" % (" ".join(got["jobs"]) or "none", " ".join(jobs) or "none"))
    hosts = ALL_HOSTS if tier in FULL_LEG_TIERS else PR_HOSTS
    if not got["tier"].startswith("ERROR") and got.get("matrix") != hosts:
        why.append("hosts %s, wanted %s" % (json.dumps(got.get("matrix")), json.dumps(hosts)))
    shown = "every job (%d), every host" % len(jobs) if tier == "full" else (" ".join(jobs) or "no job")
    shown += ", every host" if tier in FULL_LEG_TIERS and tier != "full" else ""
    say(not why, "%s: %s%s" % (name, tier, (" (" + "; ".join(why) + ")") if why else ", " + shown))


def in_order(jobs):
    return [j for j in EVERY if j in jobs]


# --- T1 the tiers by event ---------------------------------------------------
say(JOBS == WORKFLOW_JOBS, "T1 the workflow's jobs: %s" % " ".join(JOBS))
expect("T1 a push to trunk", plan(event="push"), "full", FULL_JOBS)
expect("T1 the nightly", plan(event="schedule"), NIGHTLY[0], NIGHTLY[1])
expect("T1 a dispatch, no tier", plan(event="workflow_dispatch"), "full", FULL_JOBS)
expect("T1 a dispatch, tier full", plan(event="workflow_dispatch", tier="full"), "full", FULL_JOBS)
say(plan(event="workflow_dispatch", tier="partial")["tier"].startswith("ERROR rc=2"), "T1 a dispatch's tier is full or pr, nothing else")
expect("T1 a pull request labelled full-matrix, prose only",
       plan([PROSE_PATHS[0]], event="pull_request", action="synchronize", labels="bug,full-matrix"), "full", FULL_JOBS)
expect("T1 the full-matrix label added",
       plan([PROSE_PATHS[0]], event="pull_request", action="labeled", label="full-matrix", labels="full-matrix"), "full", FULL_JOBS)
code, code_jobs = CODE_PATH
expect("T1 another label added", plan([code], event="pull_request", action="labeled", label="bug", labels="bug"), "noop", [])
expect("T1 another label added to a labelled pull request",
       plan([code], event="pull_request", action="labeled", label="bug", labels="bug,full-matrix"), "noop", [])
expect("T1 the tier input means nothing to a pull request",
       plan([code], event="pull_request", action="synchronize", tier="full"), "pr", in_order(code_jobs))

# --- T2 the pull-request tiers by path ------------------------------------------
pr = dict(event="pull_request", action="synchronize")
for p, jobs in PR_PATHS:
    expect("T2 %s" % p, plan([p], **pr), "pr", in_order(jobs))
for p in PROSE_PATHS:
    expect("T2 prose: %s" % p, plan([p], **pr), "noop", [])
expect("T2 prose alone, several", plan(PROSE_PATHS[:3], **pr), "noop", [])
expect("T2 prose beside code", plan([PROSE_PATHS[0], code, PROSE_PATHS[-1]], **pr), "pr", in_order(code_jobs))
for paths, jobs in PR_UNIONS:
    expect("T2 %s together" % " + ".join(paths), plan(paths, **pr), "pr", in_order(jobs))

# --- T3 full wins ---------------------------------------------------------------
expect("T3 a path no rule names", plan(["newplace/thing.rs"], **pr), "full", FULL_JOBS)
expect("T3 a new file at the root", plan(["build.rs"], **pr), "full", FULL_JOBS)
expect("T3 a markdown file at the root no rule names", plan(["NOTES.md"], **pr), "full", FULL_JOBS)
expect("T3 an unknown path beside prose", plan([PROSE_PATHS[0], "newplace/notes.md"], **pr), "full", FULL_JOBS)
expect("T3 an unknown path beside code", plan([code, "newplace/thing.rs"], **pr), "full", FULL_JOBS)
expect("T3 no changed paths to read", plan(**pr), "full", FULL_JOBS)
expect("T3 a change that names no path", plan([], **pr), "full", FULL_JOBS)
expect("T3 a merge that cannot be read (no repository there)",
       plan(repo=os.path.join(ROOT, "no", "such", "dir"), flags=["--merge"], **pr), "full", FULL_JOBS)
expect("T3 a dispatch, tier pr, an unknown path", plan(["newplace/thing.rs"], event="workflow_dispatch", tier="pr"), "full", FULL_JOBS)
expect("T3 a dispatch, tier pr, a branch that cannot be read",
       plan(repo=os.path.join(ROOT, "no", "such", "dir"), flags=["--merge"], event="workflow_dispatch", tier="pr"), "full", FULL_JOBS)
expect("T2 a dispatch, tier pr, code", plan([code], event="workflow_dispatch", tier="pr"), "pr", in_order(code_jobs))

# --- T6 the verdict -------------------------------------------------------------
def verdict(needs):
    r = subprocess.run([sys.executable, PLAN, "verdict"], capture_output=True, text=True, env=clean_env(NEEDS=json.dumps(needs)))
    return r.returncode, r.stdout + r.stderr


def run_of(tier, chosen, results=None, drop=(), hosts=None):
    outs = {"tier": tier, "jobs": json.dumps(chosen)}
    for k, v in (hosts if hosts is not None else (ALL_HOSTS if tier == "full" else PR_HOSTS)).items():
        outs[k] = json.dumps(v, separators=(",", ":"))
    for k in drop:  # GitHub leaves an empty output out of `needs`
        del outs[k]
    n = {"plan": {"result": "success", "outputs": outs}}
    for j in EVERY:
        n[j] = {"result": "success" if j in chosen else "skipped", "outputs": {}}
    n.update(results or {})
    return n


def vexpect(name, needs, want):
    rc, text = verdict(needs)
    say(rc == want, "T6 %s: exit %d%s" % (name, rc, "" if rc == want else " (wanted %d) %s" % (want, text.strip()[-300:])))


failed = {"result": "failure", "outputs": {}}
first, last = FULL_JOBS[0], FULL_JOBS[-1]
vexpect("a green full run", run_of("full", FULL_JOBS), 0)
vexpect("a green pr run", run_of("pr", in_order(code_jobs)), 0)
vexpect("a noop run", run_of("noop", []), 0)
vexpect("a noop run whose jobs output is absent", run_of("noop", [], drop=["jobs"]), 0)
vexpect("a chosen job failed", run_of("full", FULL_JOBS, {last: failed}), 1)
vexpect("a chosen job failed in a pr run", run_of("pr", in_order(code_jobs), {code_jobs[0]: failed}), 1)
vexpect("a chosen job was skipped", run_of("pr", in_order(code_jobs), {code_jobs[0]: {"result": "skipped", "outputs": {}}}), 1)
vexpect("a chosen job was cancelled", run_of("pr", in_order(code_jobs), {code_jobs[0]: {"result": "cancelled", "outputs": {}}}), 1)
vexpect("a job ran in a noop run", run_of("noop", [], {first: {"result": "success", "outputs": {}}}), 1)
vexpect("a full run whose jobs output is absent", run_of("full", FULL_JOBS, drop=["jobs"]), 1)
if len(FULL_JOBS) > 1:
    vexpect("a full run that chose less than every job", run_of("full", FULL_JOBS[:1]), 1)
    vexpect("a job ran unasked", run_of("pr", FULL_JOBS[:1], {FULL_JOBS[1]: {"result": "success", "outputs": {}}}), 1)
if ALL_HOSTS != PR_HOSTS:
    vexpect("a full run on the pull request's hosts", run_of("full", FULL_JOBS, hosts=PR_HOSTS), 1)
    vexpect("a pr run on every host", run_of("pr", in_order(code_jobs), hosts=ALL_HOSTS), 1)
    vexpect("a full run whose %s is absent" % sorted(ALL_HOSTS)[0], run_of("full", FULL_JOBS, drop=[sorted(ALL_HOSTS)[0]]), 1)
vexpect("a pr run that chose nothing", run_of("pr", []), 1)
vexpect("the plan failed", {"plan": {"result": "failure", "outputs": {}}, first: {"result": "skipped", "outputs": {}}}, 1)
vexpect("the plan named no tier", {"plan": {"result": "success", "outputs": {"jobs": "[]"}}, first: {"result": "skipped", "outputs": {}}}, 1)
vexpect("the plan named a tier that is none", run_of("fast", []), 1)
vexpect("the plan's jobs are no list", {"plan": {"result": "success", "outputs": {"tier": "pr", "jobs": first}}}, 1)
vexpect("a chosen job the tier job does not need", run_of("pr", [first, "lint"]), 1)

# --- T4 the wiring --------------------------------------------------------------
blocks = {}
for j in JOBS:
    m = re.search(r"^  %s:\s*\n(.*?)(?=^  [A-Za-z0-9_-]+:\s*$|\Z)" % re.escape(j), wf, re.M | re.S)
    blocks[j] = m.group(1) if m else ""
say(JOBS[:1] == ["plan"] and JOBS[-2:] == ["tier", "nightly-red"], "T4 the plan is the first job, the tier the last but the nightly's issue")
for j in EVERY:
    b = blocks[j]
    nd = re.search(r"^    needs: (plan|\[plan(, [A-Za-z0-9_-]+)*\])$", b, re.M)
    ok = nd is not None
    ok = ok and ("    if: contains(fromJSON(needs.plan.outputs.jobs || '[]'), '%s')\n" % j) in b
    say(ok, "T4 %s needs the plan and runs only when the plan names it" % j)
    out = "matrix_" + j.replace("-", "_")
    if out in ALL_HOSTS:
        say(("        include: ${{ fromJSON(needs.plan.outputs.%s || '[]') }}\n" % out) in b
            and "    runs-on: ${{ matrix.os }}\n" in b and not re.search(r"^        (os|include):\s*$|^        os: \[", b, re.M),
            "T4 %s takes its hosts from the plan" % j)
    else:
        say("matrix.os" not in b and re.search(r"^    runs-on: ubuntu-", b, re.M) is not None or j in OTHER_HOST_JOBS,
            "T4 %s has no matrix of hosts (%s)" % (j, (re.findall(r"^    runs-on: (.*)$", b, re.M) or ["?"])[0]))
reads = re.findall(r"needs\.plan\.outputs\.([a-z_]+)(\s*\|\|)?", wf)
say(len(reads) >= 3 and all(d for _, d in reads), "T4 every plan output is read with a default (%d reads: %s)" % (
    len(reads), " ".join(sorted(set(k for k, _ in reads)))))
say(all(("      %s: ${{ steps.plan.outputs.%s }}\n" % (k, k)) in blocks["plan"] for k in ["tier", "jobs"] + sorted(ALL_HOSTS)),
    "T4 the plan job declares tier, jobs and each matrix")
say('        run: python3 ci/tiers.py plan --merge --github-output "$GITHUB_OUTPUT"\n' in blocks["plan"]
    and "          fetch-depth: ${{ github.event_name == 'workflow_dispatch' && '0' || '2' }}\n" in blocks["plan"]
    and "HEAD_SHA: ${{ github.event.pull_request.head.sha }}" in blocks["plan"]
    and "IN_TIER: ${{ inputs.tier }}" in blocks["plan"],
    "T4 the plan reads the merge commit (depth 2) or a dispatch's branch (the history), and the tier input")
say("        run: python3 ci/tiers-test.py\n" in blocks["plan"]
    and blocks["plan"].index("ci/tiers-test.py") < blocks["plan"].index("ci/tiers.py plan"),
    "T4 the plan job runs this file before it plans")
t = blocks["tier"]
tn = re.findall(r"^      - ([A-Za-z0-9_-]+)\s*$", t, re.M)
say(sorted(tn) == sorted(["plan"] + EVERY), "T4 the tier job needs every job (%d of %d)" % (len(tn), len(EVERY) + 1))
ifs = re.findall(r"^    if: (.*)$", t, re.M)
say(ifs == ["${{ !cancelled() }}"], "T4 the tier job runs on !cancelled(), never always() (%s)" % "; ".join(ifs))
say("    name: tier ${{ needs.plan.outputs.tier || 'unplanned' }}\n" in t, "T4 the tier job is named for the tier")
say("        run: python3 ci/tiers.py verdict\n" in t and "          NEEDS: ${{ toJSON(needs) }}\n" in t and "actions/checkout" in t,
    "T4 the tier job's check is ci/tiers.py verdict over toJSON(needs)")
n = blocks["nightly-red"]
say("    needs: tier\n" in n and "    if: ${{ failure() && github.event_name == 'schedule' }}\n" in n
    and "      issues: write\n" in n and "        run: ci/nightly-red.sh\n" in n and "WORKFLOW: ${{ github.workflow }}" in n,
    "T4 a red nightly, and only a red nightly, runs ci/nightly-red.sh with issues: write")
say("  cancel-in-progress: ${{ github.event_name == 'pull_request' }}\n" in wf, "T4 cancel-in-progress for pull requests only")
group = re.search(r"^concurrency:\n  group: (.*)$", wf, re.M)
g = group.group(1).rstrip() if group else ""
say(g.endswith("|| format('ci-{0}-{1}', github.event_name, github.run_id) }}") and "github.ref" not in g.replace("github.head_ref", ""),
    "T4 a run that is no pull request's has a concurrency group of its own run id")
say("&& 'full' || 'pr'" in g and "format('noop-{0}', github.run_id)" in g,
    "T4 a pull request's full and pr runs are separate groups, and another label's noop cancels nothing")
say(re.search(r"^  push:\n    branches: \[%s\]\n" % re.escape(PUSH_BRANCHES), wf, re.M) is not None,
    "T4 a push runs on %s alone (a lane's push runs once, as its pull request)" % PUSH_BRANCHES)
say(re.search(r"^  schedule:\n(\s+#.*\n)*\s+- cron: ", wf, re.M) is not None, "T4 the nightly runs")
say("    types: [opened, synchronize, reopened, labeled]\n" in wf, "T4 adding a label starts a run")
say(re.search(r"^  workflow_dispatch:\n    inputs:\n(?:      [a-z_]+:\n(?:        .*\n)+)*      tier:\n", wf, re.M) is not None
    and "        options: [full, pr]\n" in wf, "T4 a dispatch takes the tier input, full or pr")
say(plan(event="push")["matrix"] == ALL_HOSTS, "T4 the full tier's hosts are the ones before tl28: %s" % json.dumps(BEFORE_HOSTS))
say(all(e in ALL_HOSTS.get(k, []) for k, v in BEFORE_HOSTS.items() for e in v) and
    all(len(ALL_HOSTS[k]) == len(v) for k, v in BEFORE_HOSTS.items()), "T4 no host left the full tier")

# --- T7 the change read from git ----------------------------------------------------
def g_(repo, *args):
    cmd = ["git", "-C", repo, "-c", "user.name=tiers-test", "-c", "user.email=tiers-test@invalid",
           "-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"] + list(args)
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout.strip()


def write(repo, rel, text):
    full = os.path.join(repo, rel)
    os.makedirs(os.path.dirname(full) or full, exist_ok=True)
    with open(full, "w", encoding="utf-8") as f:
        f.write(text)


MOVED = code + ".moved"


def merged(topic_paths):
    """A repository whose HEAD merges a topic branch (changing topic_paths)
    into a base that moved on meanwhile -> (repo, the topic's head)."""
    repo = tempfile.mkdtemp(prefix="tiers-test-")
    g_(repo, "init", "-q", "-b", "trunk")
    for rel in [code] + PROSE_PATHS[:2]:
        write(repo, rel, "one\n")
    g_(repo, "add", ".")
    g_(repo, "commit", "-q", "-m", "base")
    g_(repo, "checkout", "-q", "-b", "topic")
    for rel in topic_paths:
        write(repo, rel, "the topic\n")
    g_(repo, "add", ".")
    g_(repo, "commit", "-q", "-m", "topic")
    head = g_(repo, "rev-parse", "HEAD")
    g_(repo, "checkout", "-q", "trunk")
    write(repo, MOVED, "the base moved on\n")
    g_(repo, "add", ".")
    g_(repo, "commit", "-q", "-m", "trunk moves")
    g_(repo, "merge", "-q", "--no-ff", "-m", "merge", "topic")
    return repo, head


def dispatched(topic_paths):
    """A clone whose HEAD is a branch off trunk (changing topic_paths), its
    origin/trunk moved on meanwhile -> (the upstream, the clone)."""
    up = tempfile.mkdtemp(prefix="tiers-test-up-")
    g_(up, "init", "-q", "-b", "trunk")
    for rel in [code] + PROSE_PATHS[:1]:
        write(up, rel, "one\n")
    g_(up, "add", ".")
    g_(up, "commit", "-q", "-m", "base")
    clone = tempfile.mkdtemp(prefix="tiers-test-clone-")
    subprocess.run(["git", "clone", "-q", up, clone], check=True, capture_output=True)
    g_(clone, "checkout", "-q", "-b", "tl")
    for rel in topic_paths:
        write(clone, rel, "the branch\n")
    g_(clone, "add", ".")
    g_(clone, "commit", "-q", "-m", "branch")
    write(up, MOVED, "trunk moved on\n")
    g_(up, "add", ".")
    g_(up, "commit", "-q", "-m", "trunk moves")
    g_(clone, "fetch", "-q", "origin")
    return up, clone


made = []
try:
    repo, head = merged(PROSE_PATHS[:2])
    made.append(repo)
    expect("T7 a prose-only merge, the base moved on (its %s is not the pull request's)" % MOVED,
           plan(repo=repo, head_sha=head, flags=["--merge"], **pr), "noop", [])
    expect("T7 the same merge, the head not given", plan(repo=repo, flags=["--merge"], **pr), "noop", [])
    expect("T7 the merge's second parent is not the pull request's head",
           plan(repo=repo, head_sha="0" * 40, flags=["--merge"], **pr), "full", FULL_JOBS)
    g_(repo, "checkout", "-q", "topic")
    expect("T7 HEAD is the branch, not a merge commit", plan(repo=repo, head_sha=head, flags=["--merge"], **pr), "full", FULL_JOBS)
    repo, head = merged([PROSE_PATHS[0], code])
    made.append(repo)
    expect("T7 a merge that changes code and prose", plan(repo=repo, head_sha=head, flags=["--merge"], **pr), "pr", in_order(code_jobs))
    repo, head = merged([PROSE_PATHS[0], "somewhere/new.txt"])
    made.append(repo)
    expect("T7 a merge that adds a path no rule names", plan(repo=repo, head_sha=head, flags=["--merge"], **pr), "full", FULL_JOBS)
    dp = dict(event="workflow_dispatch", tier="pr", flags=["--merge"])
    up, clone = dispatched(PROSE_PATHS[:1])
    made += [up, clone]
    expect("T7 a dispatched prose-only branch, trunk moved on (its %s is not the branch's)" % MOVED,
           plan(repo=clone, **dp), "noop", [])
    up, clone = dispatched([code, PROSE_PATHS[0]])
    made += [up, clone]
    expect("T7 a dispatched branch that changes code", plan(repo=clone, **dp), "pr", in_order(code_jobs))
    up, clone = dispatched(["plant-unmapped.txt", code])
    made += [up, clone]
    expect("T7 a dispatched branch that adds a path no rule names (the plant)", plan(repo=clone, **dp), "full", FULL_JOBS)
    g_(clone, "checkout", "-q", "-B", "on-trunk", "origin/trunk")
    expect("T7 a dispatch at trunk itself, tier pr (no change to read)", plan(repo=clone, **dp), "full", FULL_JOBS)
    lone, _ = merged([code])
    made.append(lone)
    expect("T7 a dispatch with no origin/trunk to read", plan(repo=lone, **dp), "full", FULL_JOBS)
except (subprocess.CalledProcessError, OSError) as e:
    say(False, "T7 could not build its repositories: %s %s" % (e, getattr(e, "stderr", "")))
finally:
    for d in made:
        shutil.rmtree(d, ignore_errors=True)

# --- T8 the nightly's issue -------------------------------------------------------
FAKE_GH = r'''#!/usr/bin/env bash
printf '%s\n' "$*" >> "$FAKE_LOG"
case "$1 $2" in
  "label create") exit "${FAKE_LABEL_RC:-0}" ;;
  "issue list") printf '%s' "${FAKE_EXISTING:-}"; [ -n "${FAKE_EXISTING:-}" ] && echo; exit 0 ;;
esac
exit 0
'''


def nightly_red(existing="", label_rc=0):
    d = tempfile.mkdtemp(prefix="tiers-test-gh-")
    try:
        with open(os.path.join(d, "gh"), "w") as f:
            f.write(FAKE_GH)
        os.chmod(os.path.join(d, "gh"), 0o755)
        log = os.path.join(d, "log")
        env = dict(os.environ, PATH=d + os.pathsep + os.environ.get("PATH", ""), FAKE_LOG=log, FAKE_EXISTING=existing,
                   FAKE_LABEL_RC=str(label_rc), REPO="wolffe-lang/wolf-lsp", WORKFLOW=WORKFLOW_NAME,
                   RUN_URL="https://github.com/wolffe-lang/wolf-lsp/actions/runs/1")
        r = subprocess.run(["bash", RED], capture_output=True, text=True, env=env)
        calls = open(log).read().splitlines() if os.path.exists(log) else []
        return r.returncode, calls
    finally:
        shutil.rmtree(d, ignore_errors=True)


TITLE = "nightly red: %s on trunk" % WORKFLOW_NAME
rc, calls = nightly_red()
creates = [c for c in calls if c.startswith("issue create")]
say(rc == 0 and len(creates) == 1 and "--title %s" % TITLE in creates[0] and "--label nightly-red" in creates[0]
    and not [c for c in calls if c.startswith("issue comment")],
    "T8 no open issue: one is opened, labelled nightly-red, titled '%s'" % TITLE)
lists = [c for c in calls if c.startswith("issue list")]
say(len(lists) == 1 and "--state open" in lists[0] and "--label nightly-red" in lists[0] and TITLE in lists[0],
    "T8 the dedupe asks for an open nightly-red issue under the title")
rc, calls = nightly_red(existing="41")
say(rc == 0 and [c for c in calls if c.startswith("issue comment 41 ")] and not [c for c in calls if c.startswith("issue create")],
    "T8 an open issue: a comment on it, no new issue")
rc, calls = nightly_red(label_rc=1)
say(rc == 0 and [c for c in calls if c.startswith("issue create")], "T8 a label that already exists is no failure")
r = subprocess.run(["bash", RED], capture_output=True, text=True, env=clean_env(PATH=os.environ.get("PATH", "")))
say(r.returncode != 0, "T8 the script refuses to run without REPO, WORKFLOW and RUN_URL")
say(re.search(r"^name: %s$" % re.escape(WORKFLOW_NAME), wf, re.M) is not None, "T8 the workflow's name is %s, the title's" % WORKFLOW_NAME)

# --- T5 the tripwire: what the planner calls prose, nothing reads ---------------------
def tracked():
    try:
        return subprocess.run(["git", "-C", ROOT, "ls-files"], check=True, capture_output=True, text=True).stdout.split("\n")
    except (subprocess.CalledProcessError, FileNotFoundError):
        return []


def classes(paths):
    out = {}
    for i in range(0, len(paths), 500):
        r = subprocess.run([sys.executable, PLAN, "classify"] + paths[i:i + 500], check=True, capture_output=True, text=True)
        out.update((line.split("\t")[0], line.split("\t")[1]) for line in r.stdout.splitlines())
    return out


def patterns(p):
    """How a line names p: a directory root (`notes/`) by `notes/` or a quoted
    "notes"; a file by its path, or its own name quoted (os.path.join)."""
    if p.endswith("/"):
        d = re.escape(p[:-1])
        return [re.compile(r"(?<![A-Za-z0-9_.-])%s/" % d), re.compile(r"[\"']%s[\"']" % d)]
    return [re.compile(r"(?<![A-Za-z0-9_./-])%s(?![A-Za-z0-9_])" % re.escape(p)),
            re.compile(r"[\"']%s[\"']" % re.escape(os.path.basename(p)))]


def readers(code_files, paths):
    """{path: ["file:line", ...]}: the lines of code_files that read a file
    (READS) and name the path (patterns). Comment lines are no reader, nor a
    line RULED says names something else (its regex, with its reason)."""
    pats = {p: patterns(p) for p in paths}
    hits = {p: [] for p in paths}
    for c in code_files:
        try:
            lines = open(os.path.join(ROOT, c), encoding="utf-8", errors="replace").read().splitlines()
        except OSError:
            continue
        for n, line in enumerate(lines, 1):
            if line.lstrip().startswith(("#", "//")) or not READS.search(line):
                continue
            for p in paths:
                if any(x.search(line) for x in pats[p]) and not any(re.search(rx, line) for rx in RULED.get(p, {})):
                    hits[p].append("%s:%d" % (c, n))
    return hits


files = [f for f in tracked() if f]
if files:
    cls = classes(files)
    prose = sorted(p for p in files if cls.get(p) == "prose")
    unknown = sorted(p for p in files if cls.get(p) == "FULL")
    print("tiers-test: %d tracked paths are prose: %s" % (len(prose), " ".join(prose[:10]) + (" ..." if len(prose) > 10 else "")))
    code_files = [f for f in files if is_code(f)]
    seen = readers(code_files, MUST_FIND)
    say(all(seen[p] for p in MUST_FIND), "T5 the search fires: it finds what reads %s" % ", ".join(
        "%s (%s)" % (p, seen[p][0] if seen[p] else "NOTHING") for p in MUST_FIND))
    roots = sorted({p.split("/")[0] + ("/" if "/" in p else "") for p in prose})
    hit = {p: w for p, w in readers(code_files, roots).items() if w}
    say(not hit, "T5 no line of %d code file(s) that reads a file names a prose root (%s): %s" % (
        len(code_files), " ".join(roots), "; ".join("%s at %s" % (p, " ".join(w[:3])) for p, w in sorted(hit.items())) or "none"))
    extra_t5(files, cls, say)
    if unknown:
        print("tiers-test: WARN T5 %d tracked path(s) no rule names (a pull request touching one runs full): %s"
              % (len(unknown), " ".join(unknown[:20])))
    else:
        print("tiers-test: PASS T5 every one of %d tracked paths has a rule" % len(files))
else:
    say(False, "T5 not a git checkout: the tracked paths cannot be listed")

extra_checks(say, plan, expect, blocks, wf)

print("tiers-test: %s" % ("PASS" if fails == 0 else "FAIL (%d)" % fails))
sys.exit(1 if fails else 0)
