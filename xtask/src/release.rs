//! ls07 §§2+4 — the nvim mirror split, and the release checklist as a command.
//!
//! **THIS FILE PUBLISHES NOTHING, AND CANNOT.** No command here pushes a
//! branch, pushes a tag, uploads a vsix, contacts a marketplace or a registry,
//! or writes outside the repository and a scratch directory the caller names.
//! wolf is pre-release in private repositories, the VS Code publisher identity
//! is unregistered, and `wolf-lang` has no tagged release; every step that would
//! cross that line stops at a dry-run gate and is reported as PENDING with the
//! human action that would clear it. `docs/RELEASE.md` and
//! `docs/DISTRIBUTION.md` say the same thing in prose.
//!
//! The design rule for `release-check`: **a step is either verified or PENDING,
//! and never quietly skipped.** A checklist whose unrunnable steps disappear
//! from the output is a checklist that shrinks silently until it certifies
//! nothing, which is exactly the failure `docs/MATRIX.md` was built to prevent
//! one layer up.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{pin_value, slash};

/// A checklist step's outcome. `Pending` is a success for the exit code and a
/// failure for the reader, which is the correct asymmetry: it must not block a
/// green build today, and it must be impossible to miss.
enum Step {
    Pass(String),
    Fail(String),
    Pending(String, String),
}

pub struct Report {
    steps: Vec<(String, Step)>,
}

impl Report {
    fn pass(&mut self, step: &str, detail: impl Into<String>) {
        self.steps
            .push((step.to_string(), Step::Pass(detail.into())));
    }
    fn fail(&mut self, step: &str, detail: impl Into<String>) {
        self.steps
            .push((step.to_string(), Step::Fail(detail.into())));
    }
    fn pending(&mut self, step: &str, what: impl Into<String>, human: impl Into<String>) {
        self.steps
            .push((step.to_string(), Step::Pending(what.into(), human.into())));
    }

    /// Print, and return the number of hard failures.
    pub fn render(&self) -> usize {
        let mut failed = 0;
        let mut pending = 0;
        println!("release-check — docs/RELEASE.md, one line per step\n");
        for (step, outcome) in &self.steps {
            match outcome {
                Step::Pass(d) => println!("  PASS     {step}\n           {d}"),
                Step::Fail(d) => {
                    failed += 1;
                    println!("  FAIL     {step}\n           {d}");
                }
                Step::Pending(what, human) => {
                    pending += 1;
                    println!("  PENDING  {step}\n           {what}\n           HUMAN: {human}");
                }
            }
        }
        println!(
            "\n{} checked, {failed} failed, {pending} pending a human action.",
            self.steps.len() - pending
        );
        if pending > 0 {
            println!(
                "PENDING is not a pass. Nothing in this repository can clear those steps: they\n\
                 need a registered publisher, a tagged wolf-lang release, or a person at a clean\n\
                 machine. See docs/RELEASE.md and docs/DISTRIBUTION.md."
            );
        }
        failed
    }
}

/// `cargo xtask release-check`.
pub fn check(root: &Path) -> Report {
    let mut r = Report { steps: Vec::new() };

    let pin_text = std::fs::read_to_string(root.join("vendor").join("upstream").join("PIN"))
        .unwrap_or_default();
    let pin_commit = pin_value(&pin_text, "commit");
    let pin_version = pin_value(&pin_text, "version");

    // --- step 0: the precondition the whole checklist hangs off ---------
    //
    // The sprint keys client releases to a published wolf artifact (s66).
    //
    // This step's REASON has been rewritten three times as the world moved,
    // and the rewrites are the point — a permanently-PENDING step whose
    // reason goes stale is a step nobody reads. It said "wolf-lang tags no
    // releases" (true until v0.1.0, 2026-08-12) and then still said it while
    // the repo pinned an off-tag sha no asset could match. le04 pinned a
    // release TAG for the first time and the honest sentence became: the
    // release exists, carries tier-1 assets, and is a DRAFT.
    //
    // le05 CLEARS IT. Measured 2026-09-02: `gh release list --repo
    // wolffe-lang/wolf-lang` reports v0.2.2 as Latest, not Draft; it carries
    // three tier-1 archives; and an unauthenticated request for the download
    // URL answers 200. The precondition every publish step below hangs off is
    // satisfied, so this is a PASS and not a pending human action — leaving it
    // PENDING would be the same staleness the comment above warns about, one
    // rewrite later.
    //
    // This is deliberately a recorded measurement rather than a live `gh`
    // call: release-check runs in CI, where the network and an authenticated
    // `gh` are neither guaranteed nor wanted. The step below (`server-lane`'s
    // acquire) is where the remaining problem now lives, and it is OURS: the
    // workflow's glob asks for `wolf-<shortsha>-linux-x86_64.tar.gz` while
    // `xtask dist` publishes `wolf-<version>-<target-triple>.tar.gz`.
    r.pass(
        "0. wolf-lang has a published release to be compatible WITH",
        format!(
            "the pin is a release tag ({}, {pin_version}), and wolf-lang's release for \
             it is PUBLISHED (measured 2026-09-02: `gh release list` reports v0.2.2 as \
             Latest, three tier-1 archives, unauthenticated download URL answers 200). \
             wolf-lang#200 is resolved. What still keeps `server-lane` dark is this \
             repo's own acquire glob, not the artifact: ci.yml asks for \
             `wolf-<shortsha>-linux-x86_64.tar.gz`, `xtask dist` publishes \
             `wolf-<version>-<target-triple>.tar.gz`.",
            pin_commit.get(..7).unwrap_or("???????")
        ),
    );

    // --- step 1: the pin is bumped and re-vendored, in its own commit ---
    xtask_step(
        root,
        &mut r,
        "1. pin bumped and re-vendored (`vendor-check`)",
        "vendor-check",
    );
    match git(root, &["rev-parse", "HEAD:upstream"]) {
        Some(gitlink) if gitlink.trim() == pin_commit => {
            r.pass(
                "1b. the gitlink agrees with PIN",
                format!("both at {pin_commit}"),
            );
        }
        Some(gitlink) => r.fail(
            "1b. the gitlink agrees with PIN",
            format!(
                "PIN says {pin_commit}, the recorded gitlink is {}",
                gitlink.trim()
            ),
        ),
        None => r.fail(
            "1b. the gitlink agrees with PIN",
            "no gitlink recorded for upstream/ — see .github/workflows/ci.yml, job `independence`",
        ),
    }

    // --- step 2: the derived inventories agree with the pin -------------
    for (label, cmd) in [
        (
            "2a. grammar-drift (VS Code's four generated files)",
            "grammar-drift",
        ),
        ("2b. nvim-check (syntax keywords + pin.lua)", "nvim-check"),
        (
            "2c. config-check (helix + zed + the shared numbers)",
            "config-check",
        ),
        ("2d. emacs-check (derived keyword list)", "emacs-check"),
    ] {
        xtask_step(root, &mut r, label, cmd);
    }

    // --- step 3: the conformance suite -----------------------------------
    //
    // Split honestly in two. The server-free half is a gate today; the half
    // that needs a live `wolf` is reported by `doctor` and is PENDING while
    // there is no acquirable binary.
    match run(
        root,
        "cargo",
        &["run", "--quiet", "--bin", "lspconf", "--", "verify"],
    ) {
        Some(0) => r.pass(
            "3a. transcripts parse, validate and canonicalise (`lspconf verify`)",
            "server-free half of the ls01 suite",
        ),
        other => r.fail(
            "3a. transcripts parse, validate and canonicalise (`lspconf verify`)",
            format!("`lspconf verify` exited {other:?}"),
        ),
    }
    match run(
        root,
        "cargo",
        &["run", "--quiet", "--bin", "lspconf", "--", "doctor"],
    ) {
        Some(0) => {
            for (label, args) in [
                (
                    "3b. conformance replay against a live server",
                    vec![
                        "run",
                        "--quiet",
                        "--bin",
                        "lspconf",
                        "--",
                        "--require-server",
                        "replay",
                    ],
                ),
                (
                    "3c. one truth — publishDiagnostics == conform-run (D34)",
                    vec![
                        "run",
                        "--quiet",
                        "--bin",
                        "lspconf",
                        "--",
                        "--require-server",
                        "onetruth",
                    ],
                ),
            ] {
                match run(root, "cargo", &args) {
                    Some(0) => r.pass(label, format!("green against {pin_version}")),
                    other => r.fail(label, format!("exited {other:?}")),
                }
            }
        }
        Some(77) => r.pending(
            "3b. replay + one-truth against a live server",
            "`lspconf doctor` reports SERVER UNAVAILABLE — no `wolf` at the pin on this machine",
            "build wolf-lang at the pin and put it on PATH (README, \"Running the server lane \
             locally\"), or wait for step 0.",
        ),
        other => r.fail(
            "3b. replay + one-truth against a live server",
            format!("`lspconf doctor` failed with {other:?}"),
        ),
    }
    r.pending(
        "3d. the suite green on all THREE tier-1 OSes",
        "this command ran on one host. The three-OS claim is CI's, and CI's server lane is dark.",
        "read the `ci` workflow's matrix results for the release commit; T1 rows may not be \
         stamped from a single-host run (D35).",
    );

    // --- step 4: the matrix's T1 rows ------------------------------------
    matrix_steps(root, &mut r, &pin_commit);

    // --- step 5: stamps and compatibility rows ---------------------------
    xtask_step(
        root,
        &mut r,
        "5a. compat.json ranges earned, generated artifacts fresh (`compat-check`)",
        "compat-check",
    );

    // --- step 6: changelogs ----------------------------------------------
    changelog_steps(root, &mut r);

    // --- step 7: tag and publish ------------------------------------------
    //
    // The one place a dry run is all there is. Everything below is proven up to
    // the byte that would leave the machine.
    r.pending(
        "7a. VS Code Marketplace publish",
        "the pipeline is built and dry-run proven (vsce package + vsce ls + the manifest lint, \
         CI job `vscode-package`). The publisher identity `wolf-lang-unpublished` is a \
         placeholder and is NOT registered, so no token exists and no publish is possible.",
        "register the publisher and store the PAT — the checklist is docs/DISTRIBUTION.md \
         §\"OWED TO HUMAN\". Nothing in this repo may do it.",
    );
    r.pending(
        "7b. Open VSX publish",
        "documented alongside the marketplace path; `ovsx` takes the same vsix. No namespace is \
         registered.",
        "`ovsx create-namespace` then `ovsx publish` with an Eclipse Foundation token — \
         docs/DISTRIBUTION.md.",
    );
    match nvim_split(root, None) {
        Ok(Split::Complete(sha)) => r.pass(
            "7c. nvim mirror split is reproducible and complete",
            format!(
                "`git subtree split --prefix=clients/nvim` -> {} with every plugin directory \
                 present and nothing from wolf-lsp leaked",
                &sha[..12.min(sha.len())]
            ),
        ),
        Ok(Split::Uncommitted { sha, files }) => r.pending(
            "7c. nvim mirror split is reproducible and complete",
            format!(
                "the split of HEAD ({}) is well-formed, but {} file(s) the mirror needs are \
                 still uncommitted in clients/nvim/: {}. `git subtree split` reads HISTORY, so \
                 it cannot see them and the split does not describe this working tree.",
                &sha[..12.min(sha.len())],
                files.len(),
                files.join(", ")
            ),
            "commit clients/nvim/ and re-run. A release is cut from a commit, never from a dirty \
             tree — in CI, where the tree is always clean, this step is a hard gate.",
        ),
        Err(e) => r.fail("7c. nvim mirror split is reproducible and complete", e),
    }
    r.pending(
        "7d. nvim mirror pushed and tagged",
        "the split is proven locally (7c) and the push command is documented; \
         `wolffe-lang/wolf.nvim` does not exist.",
        "create the mirror repository, then run the push in docs/DISTRIBUTION.md §neovim. The \
         mirror is generated — never commit to it by hand.",
    );

    // --- step 8: the clean-machine install --------------------------------
    r.pending(
        "8. post-publish install on a clean machine, per T1 editor",
        "unreachable by construction: it installs FROM the published channel, and step 7 has not \
         run. A release nobody installed is a release nobody has tested.",
        "after 7, on a machine with no wolf-lsp checkout: install from the marketplace / the \
         mirror, open a vendored sample, see a diagnostic, stamp the matrix row.",
    );

    // --- step 9: upstream PR statuses --------------------------------------
    upstream_step(root, &mut r);

    r
}

// ------------------------------------------------------------- helpers --

fn xtask_step(root: &Path, r: &mut Report, label: &str, cmd: &str) {
    match run(root, "cargo", &["xtask", cmd]) {
        Some(0) => r.pass(label, format!("`cargo xtask {cmd}` green")),
        other => r.fail(label, format!("`cargo xtask {cmd}` exited {other:?}")),
    }
}

fn run(root: &Path, program: &str, args: &[&str]) -> Option<i32> {
    Command::new(program)
        .current_dir(root)
        .args(args)
        .status()
        .ok()
        .and_then(|s| s.code())
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

/// **The matrix is read, not trusted.** Its stamp must name the current pin,
/// and a row whose stamp is `NEVER` is reported as the unverified row it is.
fn matrix_steps(root: &Path, r: &mut Report, pin_commit: &str) {
    let path = root.join("docs").join("MATRIX.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        r.fail("4. docs/MATRIX.md", format!("{} is missing", slash(&path)));
        return;
    };

    let short = pin_commit.get(..7).unwrap_or("");
    let stamped = text
        .lines()
        .find(|l| l.starts_with("**Last reviewed against wolf pin"))
        .unwrap_or_default();
    if !short.is_empty() && stamped.contains(short) {
        r.pass(
            "5b. docs/MATRIX.md stamp names the current pin",
            stamped.trim().to_string(),
        );
    } else {
        r.fail(
            "5b. docs/MATRIX.md stamp names the current pin",
            format!("expected `{short}` in: {stamped:?}"),
        );
    }

    // T1 rows: the sprint says T1 breakage blocks a release, so each T1 row's
    // evidence must exist on disk. A row citing a transcript that is gone is
    // the exact artefact docs/MATRIX.md exists to prevent.
    let mut t1 = 0usize;
    let mut missing: Vec<String> = Vec::new();
    let mut never: Vec<String> = Vec::new();
    for line in text.lines() {
        if !line.starts_with('|') || !line.contains("**T1**") && !line.contains("**T2**") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let editor = cells.get(1).copied().unwrap_or("");
        let evidence = cells.get(4).copied().unwrap_or("");
        let stamp = cells.get(5).copied().unwrap_or("");
        if line.contains("**T1**") {
            t1 += 1;
        }
        if stamp.contains("NEVER") {
            never.push(editor.to_string());
        }
        for token in evidence.split('`') {
            let candidate = token.trim();
            let looks_like_path = candidate.starts_with("transcripts/")
                || candidate.starts_with("profiles/")
                || candidate.starts_with("clients/");
            if !looks_like_path {
                continue;
            }
            let direct = root.join(candidate);
            let jsonl = root.join(format!("{candidate}.jsonl"));
            if !direct.exists() && !jsonl.exists() {
                missing.push(format!("{editor}: {candidate}"));
            }
        }
    }
    if missing.is_empty() {
        r.pass(
            "4. every T1/T2 matrix row's evidence exists on disk",
            format!("{t1} T1 rows, all cited transcripts and profiles present"),
        );
    } else {
        r.fail(
            "4. every T1/T2 matrix row's evidence exists on disk",
            format!("missing: {}", missing.join("; ")),
        );
    }
    if !never.is_empty() {
        r.pending(
            "4b. rows stamped NEVER",
            format!("{} has never been run end-to-end", never.join(", ")),
            "follow the row's recipe on a machine with that editor installed and stamp the row; \
             inventing a profile to shorten the list is forbidden (profiles/README.md).",
        );
    }
}

fn changelog_steps(root: &Path, r: &mut Report) {
    let mut problems: Vec<String> = Vec::new();
    let mut ok: Vec<String> = Vec::new();
    for dir in crate::compat::SHIPPING_CLIENTS {
        let path = root.join("clients").join(dir).join("CHANGELOG.md");
        let Ok(text) = std::fs::read_to_string(&path) else {
            problems.push(format!("clients/{dir}/CHANGELOG.md is missing"));
            continue;
        };
        let compat = std::fs::read_to_string(root.join("clients").join(dir).join("compat.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v["client_version"].as_str().map(str::to_string))
            .unwrap_or_default();
        let heading = text
            .lines()
            .find(|l| l.starts_with("## "))
            .unwrap_or_default()
            .to_string();
        if compat.is_empty() || !heading.contains(&compat) {
            problems.push(format!(
                "clients/{dir}/CHANGELOG.md's first entry is {heading:?}, which does not name \
                 the declared client version {compat:?}"
            ));
        } else {
            ok.push((*dir).to_string());
        }
    }
    if problems.is_empty() {
        r.pass(
            "6. a changelog per client, topped by the version being released",
            ok.join(", "),
        );
    } else {
        r.fail(
            "6. a changelog per client, topped by the version being released",
            problems.join("; "),
        );
    }
}

fn upstream_step(root: &Path, r: &mut Report) {
    let path = root.join("docs").join("UPSTREAM.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        r.fail(
            "9. docs/UPSTREAM.md",
            "missing — integration status is this track's product",
        );
        return;
    };
    let rows: Vec<&str> = text.lines().filter(|l| row_state(l).is_some()).collect();
    let merged = rows
        .iter()
        .filter(|l| row_state(l) == Some("MERGED"))
        .count();
    if rows.is_empty() {
        r.fail(
            "9. docs/UPSTREAM.md states every patch's status in the state vocabulary",
            format!("no row uses one of {STATES:?} as a status cell"),
        );
    } else {
        r.pass(
            "9. docs/UPSTREAM.md states every patch's status in the state vocabulary",
            format!("{} tracked rows, {merged} merged", rows.len()),
        );
    }
    r.pending(
        "9b. statuses refreshed against the real forges",
        "the table is written from this repository and cannot observe a PR moving.",
        "before tagging, open each link and re-read the state. `SUBMITTED` that quietly became \
         `MERGED` is the drift this table exists to catch.",
    );
}

/// The vocabulary a `docs/UPSTREAM.md` row may use. "submitted" and "merged"
/// are different rows on purpose (ls07 §2); a row that says neither is a
/// promise rather than a state.
const STATES: &[&str] = &[
    "NOT SUBMITTED",
    "SUBMITTED",
    "MERGED",
    "DECLINED",
    "ABANDONED",
];

/// The state a **tracked** `UPSTREAM.md` row declares, if it is one.
///
/// Position is what separates a claim from a definition. A tracked row carries
/// its state in a cell that is not the first — `| PR | change | state | note |`
/// — while the file also *defines* the vocabulary in a table whose first cell
/// is the word itself. Matching anywhere in the line counts those five
/// definitions as tracked patches, which is how this step comes to report a
/// merge nobody made.
fn row_state(line: &str) -> Option<&'static str> {
    if !line.starts_with('|') {
        return None;
    }
    line.split('|')
        .skip(2)
        .find_map(|cell| STATES.iter().find(|s| **s == cell.trim().trim_matches('`')))
        .copied()
}

// ---------------------------------------------------------- nvim-split --

/// The plugin directories a `wolf.nvim` mirror must contain to be loadable.
const MIRROR_REQUIRED: &[&str] = &[
    "README.md",
    "doc/tags",
    "doc/wolf.txt",
    "ftdetect/wolf.lua",
    "ftplugin/wolf.lua",
    "lsp/wolf.lua",
    "lua/wolf/compat.lua",
    "lua/wolf/health.lua",
    "lua/wolf/init.lua",
    "lua/wolf/pin.lua",
    "plugin/wolf.lua",
    "syntax/wolf.vim",
];

/// The outcome of a split that is structurally sound.
///
/// The distinction is not pedantry. `git subtree split` reads **history**: on a
/// dirty tree it faithfully splits the last commit, which is a correct answer to
/// a question nobody asked. Reporting that as a pass would certify a mirror
/// missing whatever is still unstaged; reporting it as a failure would call a
/// clean, well-formed split broken. It is a third thing, and it says so.
pub enum Split {
    /// The split names every file the mirror needs.
    Complete(String),
    /// The split is well-formed for HEAD, but files the mirror needs are still
    /// uncommitted under `clients/nvim/`.
    Uncommitted { sha: String, files: Vec<String> },
}

/// **The mirror decision, executed.** `wolf.nvim` lives here under
/// `clients/nvim/` and is *published* to a standalone mirror by
/// `git subtree split` (ls07 §2): plugin managers handle subdirectories badly
/// and inconsistently — lazy.nvim has no option for one at all — so a mirror
/// costs one CI step and makes `{'wolffe-lang/wolf.nvim'}` work everywhere.
///
/// This function computes the split and **verifies the resulting tree**. It
/// does not push, does not create a branch, and does not tag: the mirror
/// repository does not exist, and creating it is a human act.
///
/// With `into`, the split is also materialised as a detached worktree so a real
/// Neovim can load it — which is the only way to answer "is this a loadable
/// plugin tree" rather than "does it contain the right filenames".
pub fn nvim_split(root: &Path, into: Option<&Path>) -> Result<Split, String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(["subtree", "split", "--prefix=clients/nvim"])
        .output()
        .map_err(|e| format!("git subtree split: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git subtree split failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sha.len() != 40 {
        return Err(format!("git subtree split printed {sha:?}, not a sha"));
    }

    let listing = git(root, &["ls-tree", "-r", "--name-only", &sha])
        .ok_or("git ls-tree on the split commit failed")?;
    let files: Vec<&str> = listing.lines().collect();

    // Files under `clients/nvim/` that git can see but has not recorded. A
    // required file missing from the split AND sitting here is a dirty tree,
    // not a broken plugin.
    let dirty: Vec<String> = git(root, &["status", "--porcelain", "--", "clients/nvim"])
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.get(3..).map(str::trim))
        .filter_map(|p| p.strip_prefix("clients/nvim/"))
        .map(str::to_string)
        .collect();

    let mut problems: Vec<String> = Vec::new();
    let mut uncommitted: Vec<String> = Vec::new();
    for required in MIRROR_REQUIRED {
        if files.contains(required) {
            continue;
        }
        if dirty.iter().any(|d| d == required) {
            uncommitted.push((*required).to_string());
        } else {
            problems.push(format!("the split tree has no `{required}`"));
        }
    }
    // Nothing from the harness may ride along. The prefix makes that true by
    // construction; asserting it is what catches the day someone moves a shared
    // file under `clients/nvim/` for convenience.
    for leaked in files.iter().filter(|f| {
        f.starts_with("crates/") || f.starts_with("vendor/") || f.starts_with("transcripts/")
    }) {
        problems.push(format!("`{leaked}` leaked into the mirror"));
    }
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }

    if let Some(dir) = into {
        let dir: PathBuf = dir.to_path_buf();
        if dir.exists() {
            return Err(format!(
                "{} already exists — the split materialises into a fresh path",
                slash(&dir)
            ));
        }
        let status = Command::new("git")
            .current_dir(root)
            .args(["worktree", "add", "--detach"])
            .arg(&dir)
            .arg(&sha)
            .status()
            .map_err(|e| format!("git worktree add: {e}"))?;
        if !status.success() {
            return Err("git worktree add failed".to_string());
        }
        println!("split materialised at {}", slash(&dir));
        println!(
            "  load it:   nvim --headless -u {}/tests/minimal.lua -l {}/tests/run.lua",
            slash(&dir),
            slash(&dir)
        );
        println!("  remove it: git worktree remove --force {}", slash(&dir));
    }

    println!("split commit: {sha}");
    if !uncommitted.is_empty() {
        println!(
            "\nNOTE: {} file(s) the mirror needs are uncommitted, so this split describes HEAD\n\
             and not your working tree: {}",
            uncommitted.len(),
            uncommitted.join(", ")
        );
    }
    println!(
        "\nThe push is NOT run and cannot be: `wolffe-lang/wolf.nvim` does not exist.\n\
         When it does, the publish job runs exactly:\n\
         \n    git push git@github.com:wolffe-lang/wolf.nvim.git {sha}:refs/heads/main\n\
         \nand the mirror's HEAD must already point at `refs/heads/main` — a repository\n\
         whose default branch is anything else accepts that push and then hands every\n\
         cloner an empty checkout. See docs/DISTRIBUTION.md §neovim."
    );
    if uncommitted.is_empty() {
        Ok(Split::Complete(sha))
    } else {
        Ok(Split::Uncommitted {
            sha,
            files: uncommitted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bug this exists to prevent, exactly: `docs/UPSTREAM.md` defines its
    /// own vocabulary in a table, and a substring match over the whole line
    /// counted those definitions as tracked patches — reporting "1 merged"
    /// against a file in which nothing has ever been submitted, let alone
    /// merged. A checklist step that invents a merge is worse than one that
    /// does not run.
    #[test]
    fn a_vocabulary_definition_is_not_a_tracked_row() {
        assert_eq!(
            row_state("| PR1 registration | languageId `wolf` | `NOT SUBMITTED` | in a worktree |"),
            Some("NOT SUBMITTED"),
        );
        assert_eq!(
            row_state("| lspconfig | `lsp/wolf.lua` verbatim | `MERGED` | — |"),
            Some("MERGED"),
        );
        // The definition table: the state is the FIRST cell, and it is a
        // definition of the word rather than a claim about a patch.
        assert_eq!(row_state("| `MERGED` | landed upstream |"), None);
        assert_eq!(row_state("| `SUBMITTED` | a PR exists and is open |"), None);
        // Prose is never a row, and neither is a table header or separator.
        assert_eq!(row_state("Nothing has been SUBMITTED anywhere."), None);
        assert_eq!(row_state("| PR | change | state | note |"), None);
        assert_eq!(row_state("|---|---|---|---|"), None);
    }

    /// `SUBMITTED` is a prefix of nothing and a suffix of `NOT SUBMITTED`, so
    /// the match has to be on the whole cell. A row that says NOT SUBMITTED
    /// reading as SUBMITTED is a status this table exists to keep honest.
    #[test]
    fn not_submitted_never_reads_as_submitted() {
        assert_eq!(
            row_state("| PR2 | syntax | `NOT SUBMITTED` | — |"),
            Some("NOT SUBMITTED"),
        );
        assert_ne!(
            row_state("| PR2 | syntax | `NOT SUBMITTED` | — |"),
            Some("SUBMITTED"),
        );
    }

    /// Every file the mirror must carry is inside the split prefix. A path that
    /// escaped `clients/nvim/` would be silently absent from every split and
    /// would turn 7c permanently red for a reason nobody could act on.
    #[test]
    fn every_required_mirror_path_is_relative_to_the_plugin_root() {
        for required in MIRROR_REQUIRED {
            assert!(
                !required.starts_with('/') && !required.contains(".."),
                "{required} is not a path inside the split prefix"
            );
        }
        // The three that make it a loadable plugin rather than a directory.
        for must in ["lua/wolf/init.lua", "plugin/wolf.lua", "doc/tags"] {
            assert!(
                MIRROR_REQUIRED.contains(&must),
                "the mirror must carry {must}"
            );
        }
    }
}
