//! The Neovim plugin lane, run from `cargo test`.
//!
//! Two halves, split by what they need:
//!
//! - **Artifact checks**, which need nothing at all and therefore always run:
//!   the committed help tags, and the plugin root's shape.
//! - **The plugin lane itself**, which drives a real headless Neovim over
//!   `clients/nvim/tests/`. It needs `nvim` on `PATH` and **skips loudly**
//!   when there is none, exactly the way the server-dependent suites skip
//!   without a `wolf` binary (ls00 §3). A missing editor must not turn
//!   `cargo test` red on a machine that never asked for one, and it must not
//!   pass silently either.
//!
//! Note what this lane does NOT need: a `wolf` binary. `tests/plugin_spec.lua`
//! asserts filetypes, buffer options, syntax groups, command registration,
//! config resolution, root-marker priority, tree-sitter absence handling, the
//! quickfix conversion and the whole `:checkhealth wolf` report — all of it
//! against no server, because that is the state every CI runner this repo has
//! is in. The server half is `clients/nvim/tests/smoke.lua`, which is driven
//! by hand under the capture proxy (`clients/nvim/README.md`).

use std::path::{Path, PathBuf};
use std::process::Command;

fn plugin_root() -> PathBuf {
    lsp_harness::repo_root().join("clients").join("nvim")
}

/// `nvim`, or `None` with the reason printed.
fn nvim() -> Option<String> {
    // `--version` rather than a `which`: resolving the name is not the
    // question, running it is, and a broken binary on `PATH` should read as
    // absent rather than as a mysterious later failure.
    match Command::new("nvim").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let banner = String::from_utf8_lossy(&out.stdout);
            Some(banner.lines().next().unwrap_or("nvim").trim().to_string())
        }
        _ => {
            println!(
                "SKIP: no working `nvim` on PATH — the plugin lane needs one. \
                 The plugin itself is unaffected; `clients/nvim/` is data until an \
                 editor reads it."
            );
            None
        }
    }
}

/// Run one of the plugin's headless entry points.
fn run_headless(script: &str) -> std::process::Output {
    let root = plugin_root();
    Command::new("nvim")
        .arg("--headless")
        .arg("-u")
        .arg(root.join("tests").join("minimal.lua"))
        .arg("-l")
        .arg(root.join("tests").join(script))
        // Deliberately no `WOLF_BIN`: the plugin lane is the half that must be
        // green with no toolchain, and inheriting one from the developer's
        // environment would hide a check that quietly depends on it.
        .env_remove("WOLF_BIN")
        .current_dir(lsp_harness::repo_root())
        .output()
        .expect("spawn nvim")
}

/// The whole plugin lane, as one `cargo test`.
///
/// The Lua runner prints one line per case and exits nonzero on any failure,
/// so this test's job is to surface that output when it fails — a Rust
/// assertion message that said only "exit code 1" would send the reader back
/// to the shell to find out why.
#[test]
fn the_plugin_lane_passes_in_a_real_neovim() {
    let Some(version) = nvim() else {
        return;
    };
    println!("nvim: {version}");

    let out = run_headless("run.lua");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "the Neovim plugin lane failed ({version})\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    // A runner that found no cases exits 0 with nothing to say, which is the
    // silent-green failure this whole repo is built to refuse.
    assert!(
        stdout.contains(" passed, 0 failed"),
        "the runner produced no summary — did it find any cases?\n{stdout}"
    );
    println!("{stdout}");
}

/// `doc/tags` is committed, and it carries the tags the sprint names.
///
/// Server-free and editor-free: this is a property of a file in the tree. It
/// matters because a user who installs the plugin by unpacking it never runs
/// `:helptags`, so a stale tags file makes `:h wolf-lsp` fail for exactly the
/// people least able to diagnose it. (That the tags *resolve* is asserted
/// inside Neovim, in `tests/plugin_spec.lua`; this is the half that holds on a
/// machine with no editor.)
#[test]
fn the_committed_help_tags_carry_the_named_entry_points() {
    let tags = plugin_root().join("doc").join("tags");
    let text = std::fs::read_to_string(&tags)
        .unwrap_or_else(|e| panic!("{}: {e} — run `:helptags clients/nvim/doc`", tags.display()));

    for tag in [
        "wolf.nvim",
        "wolf-lsp",
        "wolf-treesitter",
        "wolf-encoding",
        "wolf-health",
        "wolf-limitations",
    ] {
        assert!(
            text.lines().any(|l| l.split('\t').next() == Some(tag)),
            "doc/tags has no `{tag}` — the sprint names it as an entry point"
        );
    }
}

/// The plugin root holds no `.lu` file.
///
/// ls00 §5: fixtures are requested upstream, never forked locally, and the one
/// recorded exception is `fixtures/astral.lu` under `[gap.astral_plane]`.
/// `cargo xtask fixtures-check` enforces that inside `fixtures/`; nothing
/// enforced it inside a *plugin* directory, which is a new place for a
/// convenient little sample file to appear. The plugin's tests use in-memory
/// buffers and a scratch directory outside the repo for exactly this reason.
#[test]
fn the_plugin_ships_no_locally_authored_wolf_source() {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "lu" || e == "wolfi") {
                out.push(path);
            }
        }
    }

    let mut found = Vec::new();
    walk(&lsp_harness::repo_root().join("clients"), &mut found);
    assert!(
        found.is_empty(),
        "locally-authored wolf source under clients/: {found:?} — a `.lu` written \
         here is not canonical `wolf fmt` output and drifts from the corpus \
         silently. Use a corpus sample, an in-memory buffer, or a scratch \
         directory outside the repo."
    );
}
