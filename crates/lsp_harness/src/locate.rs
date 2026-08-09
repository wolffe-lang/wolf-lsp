//! Finding the `wolf` binary — and being explicit about which one won.
//!
//! Resolution order is fixed by ls00 §3: `$WOLF_BIN` → `.wolf-bin/` (the CI
//! artifact cache) → `wolf` on `PATH`. The order matters in both directions:
//! an explicit override must beat everything, and a developer's ambient
//! `wolf` must lose to the artifact CI actually downloaded.
//!
//! **This repo never builds the compiler.** Not from `upstream/`, not from a
//! vendored copy, not "just for CI". The interpreter track paid for that
//! lesson: the upstream repo is private, org policy disables deploy keys, so
//! CI cannot clone the submodule at all — and building the whole compiler per
//! job would be a multi-minute tax on a repo whose own code compiles in
//! seconds. The acquisition path is a published artifact keyed by PIN, or
//! nothing.

use std::fmt;
use std::path::{Path, PathBuf};

/// Which rule produced the binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// `$WOLF_BIN` — an explicit override, for a developer testing a build.
    Env,
    /// `.wolf-bin/` — the artifact CI downloads for the pin.
    ArtifactCache,
    /// `wolf` on `PATH` — an installed toolchain. Convenient, and the one
    /// most likely to be at the wrong version, which is why `doctor` checks.
    Path,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Source::Env => "$WOLF_BIN",
            Source::ArtifactCache => ".wolf-bin/ (artifact cache)",
            Source::Path => "PATH",
        })
    }
}

/// A resolved binary and the rule that found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub path: PathBuf,
    pub source: Source,
}

/// Platform-correct executable name. No unixisms in portable code (D35).
#[must_use]
pub fn exe_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

/// Apply the resolution order. `None` means the server is unavailable, which
/// is a normal and expected state until wolf-lang's s52 ships `wolf lsp`.
#[must_use]
pub fn locate_server(repo_root: &Path) -> Option<Located> {
    if let Some(raw) = std::env::var_os("WOLF_BIN") {
        let path = PathBuf::from(raw);
        if path.is_file() {
            return Some(Located {
                path,
                source: Source::Env,
            });
        }
    }

    let cached = repo_root.join(".wolf-bin").join(exe_name("wolf"));
    if cached.is_file() {
        return Some(Located {
            path: cached,
            source: Source::ArtifactCache,
        });
    }

    find_on_path(&exe_name("wolf")).map(|path| Located {
        path,
        source: Source::Path,
    })
}

/// Search `PATH` by hand rather than relying on the OS resolving a bare name
/// at spawn time, so `doctor` can *report the path that won*. "It works on my
/// machine" is usually a second `wolf` earlier in `PATH`.
fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_name_is_platform_correct() {
        let n = exe_name("wolf");
        if cfg!(windows) {
            assert_eq!(n, "wolf.exe");
        } else {
            assert_eq!(n, "wolf");
        }
    }

    #[test]
    fn a_repo_without_an_artifact_cache_does_not_resolve_to_one() {
        // Uses a directory that certainly has no `.wolf-bin/`; whether the
        // machine has a `wolf` on PATH is not this test's business, so it only
        // asserts the cache rule did not fire.
        let tmp = std::env::temp_dir().join("wolf-lsp-locate-test-empty");
        std::fs::create_dir_all(&tmp).unwrap();
        if let Some(found) = locate_server(&tmp) {
            assert_ne!(found.source, Source::ArtifactCache, "found {found:?}");
        }
    }

    #[test]
    fn find_on_path_reports_a_full_path_or_nothing() {
        if let Some(p) = find_on_path(&exe_name("cargo")) {
            assert!(
                p.is_absolute() || p.components().count() > 1,
                "{p:?} is a bare name"
            );
        }
    }
}
