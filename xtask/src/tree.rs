//! `cargo xtask clean-tree` (wolf-lsp#45): what the working tree holds that
//! `HEAD` does not, read so that it cannot be blind.
//!
//! The lanes' gauntlet recorded `git status --porcelain` as its clean-tree
//! check, and beside the switch script's `upstream` SYMLINK (a lane points
//! the submodule path at a tag clone) that command exits 128 —
//! `expected submodule path 'upstream' not to be a symbolic link` — with
//! nothing on stdout, so the record was empty whatever the tree held (tl24,
//! tl25). `release.rs`'s mirror check read the same command through
//! `.unwrap_or_default()`, so a failing `git` read as a clean `clients/nvim`.
//!
//! Two commands git answers beside the symlink, each with the submodule path
//! excluded: `git diff HEAD --name-only` (staged and unstaged changes to
//! tracked files) and `git ls-files --others --exclude-standard` (untracked,
//! not ignored). The gitlink itself is compared only where `upstream` is a
//! real directory. **A `git` that fails is a failure of the check, never a
//! clean tree.**

use std::path::Path;

/// The paths under `pathspec` (repo-relative, `/`-separated, sorted, each
/// once) that differ from `HEAD` or are untracked and not ignored. `Err` when
/// any `git` it runs fails. `root` must be the repository's top: git is told
/// not to look above it (`GIT_CEILING_DIRECTORIES`), so a directory that is
/// not a checkout is an error rather than a question about its parent's.
pub(crate) fn dirt(root: &Path, pathspec: &[&str]) -> Result<Vec<String>, String> {
    let _ = (root, pathspec);
    todo!("wolf-lsp#45: the clean-tree helper")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    /// A scratch directory of this test's own, removed on drop.
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn scratch(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("xtask-tree-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn git(dir: &Path, args: &[&str]) -> std::process::Output {
        Command::new("git")
            .current_dir(dir)
            .env("GIT_CEILING_DIRECTORIES", dir.parent().unwrap())
            .args(["-c", "user.name=xtask", "-c", "user.email=xtask@invalid"])
            .args(["-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
            .args(args)
            .output()
            .expect("git runs")
    }
    fn ok(dir: &Path, args: &[&str]) {
        let out = git(dir, args);
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    }

    /// A repository with one tracked file and, when asked, an `upstream`
    /// gitlink with its `.gitmodules` entry — this repository's shape.
    fn repo(dir: &Path, with_submodule: bool) {
        ok(dir, &["init", "-q"]);
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        ok(dir, &["add", "a.txt"]);
        if with_submodule {
            std::fs::write(
                dir.join(".gitmodules"),
                "[submodule \"upstream\"]\n\tpath = upstream\n\turl = https://example.invalid/upstream\n",
            )
            .unwrap();
            ok(dir, &["add", ".gitmodules"]);
            let sha = "85dab5d7e597680d6fd7034f1d7ed2acb9a92f97";
            ok(dir, &["update-index", "--add", "--cacheinfo", &format!("160000,{sha},upstream")]);
        }
        ok(dir, &["commit", "-q", "-m", "init"]);
    }

    #[cfg(unix)]
    #[test]
    fn clean_tree_sees_through_a_symlinked_submodule() {
        let s = scratch("symlink");
        let (dir, elsewhere) = (s.0.join("repo"), s.0.join("tag-clone"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        repo(&dir, true);
        // The lane's switch script: the submodule path is a symlink.
        std::os::unix::fs::symlink(&elsewhere, dir.join("upstream")).unwrap();
        std::fs::write(dir.join("a.txt"), "a, changed\n").unwrap();
        std::fs::write(dir.join("b.txt"), "b\n").unwrap();

        // The blindness, reproduced: the old check's command fails and says
        // nothing on stdout, so a record of its stdout reads clean.
        let porcelain = git(&dir, &["status", "--porcelain"]);
        assert!(!porcelain.status.success(), "git status succeeded beside the symlink");
        assert!(porcelain.stdout.is_empty());

        assert_eq!(dirt(&dir, &["."]).unwrap(), vec!["a.txt".to_string(), "b.txt".to_string()]);
        assert_eq!(dirt(&dir, &["b.txt"]).unwrap(), vec!["b.txt".to_string()]);
    }

    #[test]
    fn clean_tree_reads_a_clean_tree_as_clean() {
        let s = scratch("clean");
        repo(&s.0, true);
        assert_eq!(dirt(&s.0, &["."]).unwrap(), Vec::<String>::new());
        // An ignored file is not dirt.
        std::fs::write(s.0.join(".git/info/exclude"), "ignored.txt\n").unwrap();
        std::fs::write(s.0.join("ignored.txt"), "x\n").unwrap();
        assert_eq!(dirt(&s.0, &["."]).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn clean_tree_refuses_a_failing_git() {
        let s = scratch("norepo");
        // Not a repository (and git's ceiling stops it looking above the
        // root it is handed): the check FAILS, it does not read as clean.
        let probe = git(&s.0, &["rev-parse", "--git-dir"]);
        assert!(!probe.status.success(), "the scratch dir is inside a repository");
        let got = dirt(&s.0, &["."]);
        assert!(got.is_err(), "a failing git read as {got:?}");
    }
}
