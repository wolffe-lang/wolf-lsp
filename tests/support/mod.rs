//! Shared setup for the server-dependent suites.
//!
//! Every test in this directory needs a `wolf` binary at the pin, and none of
//! them may fail for its absence: ls00 §3's rule is that `cargo test` stays
//! green without a server and says out loud that it skipped. So each test
//! begins with [`server`] and returns early when it answers `None`, having
//! printed the reason.
//!
//! `cargo test` swallows stdout for passing tests, so a skip is invisible
//! unless someone runs with `--nocapture`. That is deliberate rather than
//! sloppy: the loud channel for "this repo tested nothing today" is the CI
//! job's own `lspconf doctor` step, which prints the verdict into the step
//! summary on every run. Duplicating it per test would bury it.

#![allow(dead_code)] // each test binary uses a different subset

use std::path::{Path, PathBuf};

use lsp_harness::profiles::Profile;
use lsp_harness::{Availability, Doctor, Session};

/// A resolved server, or `None` with the reason already printed.
pub struct Server {
    pub root: PathBuf,
    pub bin: PathBuf,
    pub pin: String,
}

impl Server {
    /// The corpus workspace every sample-based test runs in.
    #[must_use]
    pub fn samples(&self) -> PathBuf {
        self.root.join("vendor").join("upstream").join("samples")
    }

    /// The local-fixture workspace (`[gap.astral_plane]` only).
    #[must_use]
    pub fn fixtures(&self) -> PathBuf {
        self.root.join("fixtures")
    }

    /// Load a capability profile by name.
    ///
    /// # Panics
    ///
    /// If the profile is missing or invalid — that is a repo error, not a
    /// skip, and `lspconf profiles` would have caught it first.
    #[must_use]
    pub fn profile(&self, name: &str) -> Profile {
        let path = self.root.join("profiles").join(format!("{name}.json"));
        Profile::load(&path).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Spawn a session in a workspace, already initialized under `profile`.
    ///
    /// # Panics
    ///
    /// If the server cannot be spawned or refuses to initialize; both mean the
    /// binary that resolved is not a working `wolf lsp`, which `doctor` has
    /// already certified it is.
    pub fn session(&self, workspace: &Path, profile: &Profile) -> Session {
        let mut session = Session::spawn(&self.bin, workspace).expect("spawn wolf lsp");
        session
            .initialize(&profile.capabilities)
            .expect("initialize");
        session
    }
}

/// Resolve the server, or print why not.
#[must_use]
pub fn server() -> Option<Server> {
    let root = lsp_harness::repo_root();
    let doc = Doctor::run(&root);
    match &doc.availability {
        Availability::Ready { path } => Some(Server {
            root: root.clone(),
            bin: path.clone(),
            pin: doc
                .pin
                .as_ref()
                .map(|p| p.commit.clone())
                .unwrap_or_default(),
        }),
        other => {
            println!("SKIP: {}", other.reason(doc.pin.as_ref()));
            None
        }
    }
}

/// Read a corpus sample.
///
/// # Panics
///
/// If the file is missing — `cargo xtask vendor-check` guarantees every path
/// in `samples.toml` is on disk, so a failure here is drift, not a skip.
#[must_use]
pub fn read(workspace: &Path, name: &str) -> Vec<u8> {
    let path = workspace.join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}
