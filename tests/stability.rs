//! §3 — stability demonstrated, not asserted.
//!
//! The sprint's acceptance criterion, run rather than promised:
//!
//! > a deliberate red PR adding a new optional field to a server response
//! > leaves the suite green; one changing a diagnostic span turns it red.
//! > Both exercised, then reverted.
//!
//! This repo cannot patch the compiler, so the deliberate change is made to
//! the **transcript** and replayed against the untouched server. That inverts
//! the direction and asserts exactly the same property:
//!
//! - *the server gains an optional field* ⇔ the live message has a member the
//!   transcript does not. Removing a member from a recorded response produces
//!   precisely that situation, and `subset` must stay green.
//! - *the server moves a diagnostic span* ⇔ the live range differs from the
//!   recorded one. Shifting a recorded range by one column produces precisely
//!   that, and the suite must turn red.
//!
//! "Then reverted" is structural: every mutation is written to a scratch copy
//! under `target/`, never to the committed library, so there is nothing to
//! forget to revert.

mod support;

use std::path::{Path, PathBuf};

use lsp_transcript::{jsonl, record::Transcript};
use serde_json::Value;

/// Copy a committed transcript (and its script, which carries the spawn
/// environment) into a scratch directory and hand back the mutable copy.
fn scratch(server: &support::Server, name: &str, tag: &str) -> (PathBuf, Transcript) {
    let source = server.root.join("transcripts").join(name);
    let dir = server.root.join("target").join("stability").join(tag);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let dest = dir.join(source.file_name().expect("the transcript has a file name"));
    let text = std::fs::read_to_string(&source).unwrap_or_else(|e| panic!("{name}: {e}"));
    // The script travels with it: `replay` reads `env` from the `.lsps`
    // beside the transcript, and a copy without one would spawn a server
    // without the slow-query knob its cancellation records depend on.
    let script = source.with_extension("lsps");
    if script.is_file() {
        std::fs::copy(&script, dest.with_extension("lsps")).expect("copy script");
    }
    let transcript = jsonl::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
    (dest, transcript)
}

fn write(path: &Path, transcript: &Transcript) {
    std::fs::write(path, jsonl::to_string(transcript)).expect("write scratch transcript");
}

fn replay(server: &support::Server, path: &Path) -> lsp_harness::replay::Report {
    lsp_harness::replay::replay(&server.root, &server.bin, path, &server.pin)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn the_unmodified_transcript_is_green_first() {
    // The control. Without it, a green result below could mean "stable" or
    // "the replay never ran".
    let Some(server) = support::server() else {
        return;
    };
    let (path, transcript) = scratch(&server, "diagnostics/broken-e0501.jsonl", "control");
    write(&path, &transcript);
    let report = replay(&server, &path);
    assert!(
        report.ok(),
        "the control replay failed: {:?}",
        report.mismatches
    );
    assert!(report.compared > 0, "the control compared nothing");
}

#[test]
fn a_server_that_gains_an_optional_field_leaves_the_suite_green() {
    // Simulated by removing a field from the recorded `initialize` response:
    // the live server still sends it, so the live message is a strict superset
    // of the transcript — exactly the shape of a server that grew a capability.
    //
    // `ServerCapabilities` is the single most important `subset` default in
    // `lsp_transcript::defaults`, and this is what it buys.
    let Some(server) = support::server() else {
        return;
    };
    let (path, mut transcript) = scratch(&server, "lifecycle/initialize-minimal.jsonl", "gained");

    let mut removed = None;
    for rec in &mut transcript.records {
        if let Some(caps) = rec
            .result
            .as_mut()
            .and_then(|r| r.pointer_mut("/capabilities"))
            .and_then(Value::as_object_mut)
        {
            // Drop a real advertised capability. If the suite were comparing
            // exactly, the server still sending it would be a mismatch.
            removed = caps.remove("hoverProvider").map(|v| ("hoverProvider", v));
            break;
        }
    }
    let (name, value) = removed.expect("the initialize response advertises hoverProvider");
    assert_eq!(value, Value::Bool(true), "the capability being dropped");
    write(&path, &transcript);

    let report = replay(&server, &path);
    assert!(
        report.ok(),
        "a transcript that expects LESS than the server sends turned the suite red — \
         forward compatibility is broken and every capability the server ever gains \
         will now be a forty-file diff: {:#?}",
        report.mismatches
    );
    let _ = name;
}

#[test]
fn a_server_that_moves_a_diagnostic_span_turns_the_suite_red() {
    // The other half, and the one that matters: a span is behavior. Shifting
    // the recorded range one column to the right must be caught, because a
    // real one-column shift is what a wrong position encoding produces and it
    // is invisible to a human reading the JSON.
    let Some(server) = support::server() else {
        return;
    };
    let (path, mut transcript) = scratch(&server, "diagnostics/broken-e0501.jsonl", "moved");

    let mut moved = false;
    for rec in &mut transcript.records {
        let Some(diagnostics) = rec
            .params
            .as_mut()
            .and_then(|p| p.pointer_mut("/diagnostics"))
            .and_then(Value::as_array_mut)
        else {
            continue;
        };
        for diagnostic in diagnostics.iter_mut() {
            if let Some(ch) = diagnostic
                .pointer("/range/start/character")
                .and_then(Value::as_u64)
            {
                diagnostic["range"]["start"]["character"] = Value::from(ch + 1);
                moved = true;
            }
        }
    }
    assert!(moved, "no diagnostic range to move — the fixture changed");
    write(&path, &transcript);

    let report = replay(&server, &path);
    assert!(
        !report.ok(),
        "a diagnostic moved by one column and the suite stayed green. Every squiggle in \
         every editor could be one character off and this harness would not notice."
    );
    let complaint = report
        .mismatches
        .iter()
        .map(ToString::to_string)
        .collect::<String>();
    assert!(
        complaint.contains("publishDiagnostics"),
        "the failure should name the notification that moved: {complaint}"
    );
}

#[test]
fn a_reordered_diagnostics_array_stays_green() {
    // Order within `diagnostics` is not specified by LSP and not asserted
    // here: the default is a multiset. A suite that failed on reordering
    // would go red the first time the server's worker threads finished in a
    // different order, which is a Tuesday.
    let Some(server) = support::server() else {
        return;
    };
    let (path, mut transcript) = scratch(&server, "diagnostics/broken-e0501.jsonl", "reordered");

    for rec in &mut transcript.records {
        if let Some(related) = rec
            .params
            .as_mut()
            .and_then(|p| p.pointer_mut("/diagnostics/0/relatedInformation"))
            .and_then(Value::as_array_mut)
        {
            assert!(related.len() > 1, "need two secondaries to reorder");
            related.reverse();
        }
    }
    write(&path, &transcript);
    let report = replay(&server, &path);
    assert!(
        report.ok(),
        "reordering related information turned the suite red: {:#?}",
        report.mismatches
    );
}

#[test]
fn a_changed_diagnostic_code_turns_the_suite_red() {
    // Codes are the most load-bearing thing a diagnostic carries — the corpus
    // pins `fail(E0501)`, `wolf --explain` keys off it, and every downstream
    // tool matches on it. A suite that let a code change through would let the
    // one-truth claim rot silently.
    let Some(server) = support::server() else {
        return;
    };
    let (path, mut transcript) = scratch(&server, "diagnostics/broken-e0501.jsonl", "recoded");

    let mut changed = false;
    for rec in &mut transcript.records {
        if let Some(diagnostics) = rec
            .params
            .as_mut()
            .and_then(|p| p.pointer_mut("/diagnostics"))
            .and_then(Value::as_array_mut)
        {
            for diagnostic in diagnostics.iter_mut() {
                if diagnostic.get("code").is_some() {
                    diagnostic["code"] = Value::from("E9999");
                    changed = true;
                }
            }
        }
    }
    assert!(changed, "no diagnostic code to change");
    write(&path, &transcript);

    let report = replay(&server, &path);
    assert!(
        !report.ok(),
        "a diagnostic code changed and the suite stayed green"
    );
}

#[test]
fn a_changed_formatting_result_turns_the_suite_red() {
    // Formatting is a byte-for-byte claim (`wolf fmt` is the one canonical
    // style), so its matcher is `exact` and a single character must be caught.
    let Some(server) = support::server() else {
        return;
    };
    let (path, mut transcript) = scratch(
        &server,
        "requests/formatting-restores-canonical.jsonl",
        "reformatted",
    );

    let mut changed = false;
    for rec in &mut transcript.records {
        if let Some(edits) = rec.result.as_mut().and_then(Value::as_array_mut) {
            for edit in edits.iter_mut() {
                if let Some(text) = edit.get("newText").and_then(Value::as_str) {
                    edit["newText"] = Value::from(format!("{text} "));
                    changed = true;
                }
            }
        }
    }
    assert!(changed, "no formatting edit to change");
    write(&path, &transcript);

    let report = replay(&server, &path);
    assert!(
        !report.ok(),
        "a trailing space appeared in the formatter's output and the suite stayed green"
    );
}
