//! Ruling #38 = C (wolf-lsp#17): helix's two teardown shapes, replayed live.
//!
//! The committed helix capture is the 19-record shape and sits at an older
//! pin, so `lspconf replay` skips it by name and CI never sees either shape
//! run. These tests take its records, re-head them at THIS pin in a scratch
//! directory (tl03 and every re-capture since measured helix's re-captures as
//! header-only), and replay both shapes against the acquired server:
//!
//! - **shape one**, 19 records, ends on the `formatting` response;
//! - **shape two**, the same plus run B's record 20: helix's `shutdown`
//!   request, unanswered in the capture.
//!
//! What is asserted is the TEARDOWN, not the 19 records' payloads: those are
//! the smoke's own claim, re-earned by a re-capture (wolf-lsp#26), and a stale
//! hover string there is not this ruling's business. The trailing `shutdown`
//! is: replay sends it live and the server owes it `"result": null`.

mod support;

use std::path::PathBuf;

use lsp_harness::replay::{self, Teardown};

const RUN_B_RECORD_20: &str =
    r#"{"dir":"c2s","id":7,"kind":"request","method":"shutdown","seq":20}"#;

/// The committed helix capture re-headed at `pin`, with or without record 20,
/// written under a per-test scratch directory.
fn helix_at_pin(server: &support::Server, tag: &str, with_shutdown: bool) -> PathBuf {
    let committed = server.root.join("transcripts/helix/smoke.jsonl");
    let text = std::fs::read_to_string(&committed).expect("the committed helix capture");
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut header: serde_json::Value = serde_json::from_str(&lines[0]).expect("header");
    header["wolf_pin"] = serde_json::Value::from(server.pin.clone());
    lines[0] = serde_json::to_string(&header).expect("header serializes");
    if with_shutdown {
        lines.push(RUN_B_RECORD_20.to_string());
    }
    let dir = std::env::temp_dir().join(format!("wolf-lsp-teardown-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("smoke.jsonl");
    std::fs::write(&path, lines.join("\n") + "\n").expect("write");
    path
}

#[test]
fn helix_shape_one_replays_and_reports_no_handshake() {
    let Some(server) = support::server() else {
        return;
    };
    let path = helix_at_pin(&server, "one", false);
    let report =
        replay::replay(&server.root, &server.bin, &path, &server.pin).expect("replay runs");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
    assert_eq!(report.teardown, Some(Teardown::NoHandshake));
    assert!(
        report.mismatches.iter().all(|f| f.method != "shutdown"),
        "{:?}",
        report.mismatches
    );
}

#[test]
fn helix_shape_two_replays_its_trailing_shutdown_live() {
    let Some(server) = support::server() else {
        return;
    };
    let path = helix_at_pin(&server, "two", true);
    let report =
        replay::replay(&server.root, &server.bin, &path, &server.pin).expect("replay runs");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
    assert_eq!(report.teardown, Some(Teardown::TrailingShutdown));
    let shutdown: Vec<_> = report
        .mismatches
        .iter()
        .filter(|f| f.method == "shutdown")
        .collect();
    assert!(
        shutdown.is_empty(),
        "the live server's answer: {shutdown:?}"
    );
}
