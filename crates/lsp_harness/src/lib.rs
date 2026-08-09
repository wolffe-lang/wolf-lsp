//! `lsp_harness` — everything that touches a *process*: framing, binary
//! resolution, the pin, and the diagnosis of whether a server exists at all.
//!
//! **There is no language-server behavior here, and there never will be.**
//! Not a fallback, not a "temporary" diagnostic parser, not a formatting shim.
//! The server is `wolf lsp` — the compiler itself (D34, one process, one
//! truth) — and a second implementation of anything server-side in this repo
//! is the exact failure this track exists to avoid. A capability an editor
//! wants is a wolf-lang sprint, not a shim here.
//!
//! Spawn and replay land in ls01, on top of [`framing`]. What ls00 owns is the
//! half that runs *without* a server, plus the loud, reasoned skip for the
//! half that does not: see [`doctor`].

pub mod doctor;
pub mod framing;
pub mod locate;
pub mod pin;

pub use doctor::{Availability, Doctor};
pub use locate::{Located, Source, locate_server};
pub use pin::Pin;

/// Everything matched.
pub const EXIT_OK: i32 = 0;
/// The comparison ran and something did not match — a real finding.
pub const EXIT_MISMATCH: i32 = 1;
/// The harness itself could not run: bad flags, missing file, corrupt pin.
/// Mirrors the compiler and interpreter convention, so a caller can tell "the
/// server is wrong" from "the tool is wrong".
pub const EXIT_HARNESS_ERROR: i32 = 2;
/// Skipped: no server at the pin. Distinct from success on purpose — a CI lane
/// that reports 0 for "did nothing" is a lane nobody notices is dark.
pub const EXIT_SKIPPED: i32 = 77;

/// Locate the repository root by walking up from `start` looking for the
/// marker files this repo is guaranteed to have.
///
/// Tests and the binary both need this, and both may run from an arbitrary
/// cwd. Looking for `Cargo.toml` alone would stop at a member crate.
#[must_use]
pub fn find_repo_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        if dir.join("vendor").join("upstream").is_dir() && dir.join("Cargo.toml").is_file() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }
    None
}

/// The repo root as seen from this crate's source location — correct from any
/// cwd, which is what tests need.
#[must_use]
pub fn repo_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    find_repo_root(manifest).unwrap_or_else(|| manifest.to_path_buf())
}

/// Upstream data root: the live submodule when someone has initialized it,
/// else the tracked vendored snapshot.
///
/// Mirrors `wolf_interp::upstream_root`. CI always takes the second branch —
/// the submodule is private and org policy disables deploy keys, so CI cannot
/// clone it (see `vendor/README.md`). Local checkouts may have either.
#[must_use]
pub fn upstream_root(repo_root: &std::path::Path) -> std::path::PathBuf {
    let live = repo_root.join("upstream").join("spec");
    if live.is_dir() {
        repo_root.join("upstream")
    } else {
        repo_root.join("vendor").join("upstream")
    }
}

/// Render a path with `/` separators on every platform.
///
/// Transcripts travel between machines and get diffed; a Windows `\` in a
/// `uri` or a workspace path would make identical sessions compare unequal.
/// Mirrors `wolf_interp::slash_path`, for the same reason.
/// Joining components with `/` naively doubles the leading separator on unix,
/// where the root component is itself `/` — `//home/dev/...`. Handle it.
#[must_use]
pub fn slash_path(path: &std::path::Path) -> String {
    let mut out = String::new();
    for component in path.components() {
        let text = component.as_os_str().to_string_lossy();
        if text == "/" || text == "\\" {
            out.push('/');
            continue;
        }
        if !out.is_empty() && !out.ends_with('/') {
            out.push('/');
        }
        out.push_str(&text);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_root_is_findable_from_the_crate() {
        let root = repo_root();
        assert!(
            root.join("vendor/upstream/PIN").is_file(),
            "{}",
            root.display()
        );
    }

    #[test]
    fn upstream_root_falls_back_to_the_vendored_snapshot() {
        let root = repo_root();
        let up = upstream_root(&root);
        assert!(
            up.join("spec").join("grammar.ebnf").is_file(),
            "{}",
            up.display()
        );
    }

    #[test]
    fn slash_path_never_leaks_a_platform_separator() {
        let p = std::path::Path::new("vendor")
            .join("upstream")
            .join("samples");
        assert_eq!(slash_path(&p), "vendor/upstream/samples");
    }

    #[test]
    fn slash_path_does_not_double_the_root_separator() {
        // `components()` yields the root as its own `/`; joining naively gives
        // `//home/…`, which is a different string to anything comparing URIs.
        let abs = std::path::Path::new("/home/dev/wolf-lsp/vendor");
        assert_eq!(slash_path(abs), "/home/dev/wolf-lsp/vendor");
    }
}
