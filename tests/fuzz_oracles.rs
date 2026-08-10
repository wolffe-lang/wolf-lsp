//! §7 — the fuzzer's three oracles, and the demonstration that they are not
//! vacuous.
//!
//! An oracle that has only ever been run against a server that passes is an
//! oracle nobody has seen work. Sprint acceptance asks for both the round-trip
//! and the sync-mode-equivalence oracle to *demonstrably catch a planted bug*,
//! so each one is exercised twice here: once on the path that should hold, and
//! once with a deliberate defect that must be caught.
//!
//! The bug is planted in the **edit sequence**, not in the server — this repo
//! cannot patch the compiler, and would not want to. A round-trip that does not
//! actually return to the original bytes is exactly the shape of the bug the
//! oracle exists to find (overlay state that survives an edit and its undo),
//! and running it against the live server proves the comparison discriminates.
//!
//! The CI budget is the sprint's: a seeded deterministic run on every PR, the
//! long sweep nightly. Both are the same code with a different splice count.

mod support;

use std::time::Instant;

use lsp_harness::fuzz::{self, Rng};
use serde_json::json;

/// Splices per PR-tier session. The nightly job runs `lspconf fuzz` with a
/// much larger count and a rotating seed; this is the floor that must hold on
/// every commit without adding a minute to the gate.
const PR_SPLICES: usize = 24;

fn subject(server: &support::Server) -> (std::path::PathBuf, &'static str) {
    (server.samples(), "regions.lu")
}

/// The offset of the `;` that stands alone on its line — the one
/// `grammar/semicolon.lu` exists to diagnose.
fn stray_semicolon(src: &[u8]) -> usize {
    src.windows(6)
        .position(|w| w == b"\n    ;")
        .map(|i| i + 5)
        .expect("grammar/semicolon.lu no longer holds a `;` on its own line")
}

#[test]
fn a_seeded_session_holds_all_three_oracles() {
    let Some(server) = support::server() else {
        return;
    };
    let (workspace, sample) = subject(&server);
    let profile = server.profile("minimal");
    for seed in [1u64, 2, 3] {
        let outcome = fuzz::run(&server.bin, &workspace, sample, &profile, seed, PR_SPLICES)
            .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        assert!(
            outcome.ok(),
            "seed {seed} failed an oracle:\n  {}\nreproduce: \
             lspconf fuzz {sample} --seed {seed} --splices {PR_SPLICES}",
            outcome.failures.join("\n  ")
        );
    }
}

#[test]
fn the_same_seed_produces_the_same_session() {
    // Reproducibility is the whole value of a seeded fuzzer: a failure in CI
    // has to be reproducible on the machine that fixes it. Path-derived
    // seeding is forbidden (ls00 §4) for the same reason.
    let Some(server) = support::server() else {
        return;
    };
    let (workspace, sample) = subject(&server);
    let profile = server.profile("minimal");
    let a = fuzz::run(&server.bin, &workspace, sample, &profile, 99, 8).expect("first");
    let b = fuzz::run(&server.bin, &workspace, sample, &profile, 99, 8).expect("second");
    assert_eq!(a.history, b.history, "the same seed diverged");
}

#[test]
fn the_round_trip_oracle_catches_a_planted_bug() {
    // Planted defect: the "undo" is one byte short, so the buffer does NOT
    // return to its original bytes. A round-trip oracle that compared nothing
    // — or that compared the wrong thing — would still report success.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let path = workspace.join("grammar/semicolon.lu");
    let original = support::read(&workspace, "grammar/semicolon.lu");
    let uri = lsp_harness::session::file_uri(&path);

    let mut session = server.session(&workspace, &profile);
    fuzz::open(&mut session, &uri, &original, 1).expect("open");
    let initial = fuzz::read_state(&mut session, &uri).expect("initial state");

    // The subject's whole reason for existing is a stray `;` that raises
    // E0002. Delete it — the diagnostic goes away — and then "restore" it as a
    // space instead. The final buffer differs from the original by one byte and
    // by one diagnostic, which is exactly what a leaked-overlay bug looks like.
    // The *stray* `;` — the one on its own line. `position(|b| b == b';')`
    // would find the one inside the header comment instead, and editing a
    // comment changes no diagnostic, which is how this demonstration was
    // vacuous on its first draft.
    let semi = stray_semicolon(&original);
    assert_eq!(
        initial.diagnostics.len(),
        1,
        "the subject is supposed to start with exactly one diagnostic"
    );

    let broken = lsp_harness::drive::splice(&original, semi, semi + 1, b"");
    fuzz::change(&mut session, &uri, &broken, 2).expect("edit");
    let bad_undo = lsp_harness::drive::splice(&broken, semi, semi, b" ");
    fuzz::change(&mut session, &uri, &bad_undo, 3).expect("bad undo");
    let after = fuzz::read_state(&mut session, &uri).expect("state after");

    assert_ne!(
        bad_undo, original,
        "the planted bug did not actually change the bytes — the demonstration would be \
         vacuous"
    );
    assert_ne!(
        after.diagnostics, initial.diagnostics,
        "the round-trip oracle compared two states that differ and saw no difference"
    );
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn the_round_trip_oracle_passes_on_a_correct_undo() {
    // The other half: the same comparison must NOT fire when the bytes really
    // do come back. Without this, the test above would be satisfied by an
    // oracle that always reports a difference.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let path = workspace.join("grammar/semicolon.lu");
    let original = support::read(&workspace, "grammar/semicolon.lu");
    let uri = lsp_harness::session::file_uri(&path);

    let mut session = server.session(&workspace, &profile);
    fuzz::open(&mut session, &uri, &original, 1).expect("open");
    let initial = fuzz::read_state(&mut session, &uri).expect("initial state");

    // The *stray* `;` — the one on its own line. `position(|b| b == b';')`
    // would find the one inside the header comment instead, and editing a
    // comment changes no diagnostic, which is how this demonstration was
    // vacuous on its first draft.
    let semi = stray_semicolon(&original);
    let broken = lsp_harness::drive::splice(&original, semi, semi + 1, b"");
    fuzz::change(&mut session, &uri, &broken, 2).expect("edit");
    let restored = lsp_harness::drive::splice(&broken, semi, semi, b";");
    assert_eq!(restored, original, "the undo must be exact for this half");
    fuzz::change(&mut session, &uri, &restored, 3).expect("undo");
    let after = fuzz::read_state(&mut session, &uri).expect("state after");

    assert_eq!(
        after.diagnostics, initial.diagnostics,
        "identical bytes produced different diagnostics"
    );
    assert_eq!(after.symbols, initial.symbols);
    assert_eq!(after.formatting, initial.formatting);
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn the_sync_mode_equivalence_oracle_catches_a_planted_bug() {
    // Planted defect: the two delivery paths do not actually end at the same
    // text. The oracle must notice — otherwise "the path taken to a document
    // does not change the answer" is an untested slogan, and it is the exact
    // property fackr (full text every keystroke) and facsimile (500 ms
    // debounce, version pinned to 1) differ on.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let path = workspace.join("hello.lu");
    let original = support::read(&workspace, "hello.lu");
    let uri = lsp_harness::session::file_uri(&path);

    let mut session = server.session(&workspace, &profile);
    fuzz::open(&mut session, &uri, &original, 1).expect("open");

    let one_shot = b"fn main() -> !int {\n    let a = 1 < 2 < 3\n    0\n}\n".to_vec();
    fuzz::change(&mut session, &uri, &one_shot, 2).expect("one shot");
    let at_once = fuzz::read_state(&mut session, &uri).expect("state");

    // A keystroke storm that ends somewhere ELSE.
    for (version, step) in (3..).zip([
        "fn main",
        "fn main() -> !int {",
        "fn main() -> !int {\n    0\n}\n",
    ]) {
        fuzz::change(&mut session, &uri, step.as_bytes(), version).expect("storm");
    }
    let by_storm = fuzz::read_state(&mut session, &uri).expect("state");

    assert_ne!(
        at_once.diagnostics, by_storm.diagnostics,
        "the sync-mode oracle compared two genuinely different documents and saw no \
         difference — it would not catch a real path-dependence bug either"
    );
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn the_sync_mode_equivalence_oracle_passes_when_both_paths_agree() {
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let path = workspace.join("hello.lu");
    let original = support::read(&workspace, "hello.lu");
    let uri = lsp_harness::session::file_uri(&path);
    let target = b"fn main() -> !int {\n    let a = 1 < 2 < 3\n    0\n}\n".to_vec();

    let mut session = server.session(&workspace, &profile);
    fuzz::open(&mut session, &uri, &original, 1).expect("open");
    fuzz::change(&mut session, &uri, &target, 2).expect("one shot");
    let at_once = fuzz::read_state(&mut session, &uri).expect("state");

    // Reach the same bytes one prefix at a time — fackr's actual behavior.
    fuzz::change(&mut session, &uri, &original, 3).expect("reset");
    for (version, len) in (4..).zip([8, 20, 32, target.len()]) {
        fuzz::change(&mut session, &uri, &target[..len], version).expect("storm");
    }
    let by_storm = fuzz::read_state(&mut session, &uri).expect("state");

    assert_eq!(
        at_once.diagnostics, by_storm.diagnostics,
        "the same final text produced different diagnostics depending on how it arrived"
    );
    assert_eq!(at_once.symbols, by_storm.symbols);
    assert_eq!(at_once.formatting, by_storm.formatting);
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn a_range_carrying_change_against_a_full_sync_server_does_not_corrupt_state() {
    // The declared `textDocumentSync` must be the one honored. A server that
    // quietly accepted deltas it never advertised would corrupt the buffer for
    // any client that sent them — and the client would be within its rights,
    // because it read the capability.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let path = workspace.join("hello.lu");
    let original = support::read(&workspace, "hello.lu");
    let uri = lsp_harness::session::file_uri(&path);

    let mut session = server.session(&workspace, &profile);
    fuzz::open(&mut session, &uri, &original, 1).expect("open");
    let before = fuzz::read_state(&mut session, &uri).expect("state");

    session
        .notify(
            "textDocument/didChange",
            json!({"textDocument": {"uri": uri, "version": 2},
                   "contentChanges": [{"range": {"start": {"line": 0, "character": 0},
                                                 "end": {"line": 0, "character": 0}},
                                       "rangeLength": 0, "text": "GARBAGE"}]}),
        )
        .expect("delta");
    let after = fuzz::read_state(&mut session, &uri).expect("state");
    assert_eq!(
        before.diagnostics, after.diagnostics,
        "a range-carrying contentChange moved a FULL-sync server's state"
    );
    assert_eq!(before.formatting, after.formatting);
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn the_generator_is_reproducible_without_a_server() {
    // The one test in this file that needs nothing running: seeding is a
    // property of the harness, and it must hold in the serverless half of CI
    // too, or a nightly failure would be unreproducible on a laptop with no
    // compiler build.
    let src = b"fn main() -> !int {\n    let s = \"h\xc3\xa9llo\"\n    0\n}\n";
    let run = |seed: u64| {
        let mut rng = Rng::new(seed);
        let mut text = src.to_vec();
        (0..64)
            .map(|_| {
                let s = fuzz::generate(&mut rng, &text);
                text = s.apply(&text);
                s
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(12345), run(12345));
    assert_ne!(run(12345), run(54321));

    let _ = Instant::now();
}
