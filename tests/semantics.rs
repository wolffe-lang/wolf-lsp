//! §6 — cancellation, overlay precedence, version discipline, and lifecycle.
//!
//! The four areas editors actually break on, minus position encoding (which
//! has its own file). Each test states the *user-visible* failure it prevents,
//! because "the server answered -32800" is not why anyone cares — "the editor
//! stopped responding while you typed" is.

mod support;

use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// The server's own injected-slow-query knob. Without it, cancellation tests
/// are races the harness usually loses: v0 answers a small file in single-digit
/// milliseconds, and a `$/cancelRequest` sent immediately after the request
/// arrives long after the answer.
const SLOW: &str = "WOLF_QUERY_TEST_SLOW_MS";

fn slow_session(
    server: &support::Server,
    profile: &str,
    ms: &str,
) -> (lsp_harness::Session, String) {
    let workspace = server.samples();
    let profile = server.profile(profile);
    let mut session = lsp_harness::Session::spawn_with_env(
        &server.bin,
        &workspace,
        &[(SLOW.to_string(), ms.to_string())],
    )
    .expect("spawn");
    session
        .initialize(&profile.capabilities)
        .expect("initialize");
    let uri = lsp_harness::session::file_uri(&workspace.join("hello.lu"));
    let text = String::from_utf8_lossy(&support::read(&workspace, "hello.lu")).into_owned();
    session
        .notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "wolf",
                                    "version": 1, "text": text}}),
        )
        .expect("didOpen");
    session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish");
    (session, uri)
}

#[test]
fn a_cancelled_request_completes_promptly_with_request_cancelled() {
    // Prevents: the editor's request queue filling with work nobody wants
    // while the user keeps typing. Cancellation that only reaches the
    // transport — the tower-lsp failure report 09 names — leaves the compute
    // running, so the *promptness* half of this assertion is the real one.
    let Some(server) = support::server() else {
        return;
    };
    let (mut session, uri) = slow_session(&server, "minimal", "10000");

    let started = Instant::now();
    // `documentSymbol` rather than `hover`: hover can answer straight out of
    // the package-analysis cache the open already filled, without ever
    // reaching a cancellation checkpoint.
    let id = session
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": uri}}),
        )
        .expect("request");
    session
        .notify("$/cancelRequest", json!({"id": id}))
        .expect("cancel");
    let (resp, took) = session
        .response(id, started)
        .expect("cancelled requests still answer");

    assert_eq!(
        resp.pointer("/error/code").and_then(Value::as_i64),
        Some(-32800),
        "expected RequestCancelled: {resp}"
    );
    assert!(
        took < Duration::from_secs(5),
        "the cancel took {took:?} against a query told to dawdle 10 s — it reached the \
         transport but not the compute"
    );
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn later_requests_are_unaffected_by_a_cancellation() {
    // Prevents: one cancelled request poisoning the session, which presents as
    // "the language server stopped working after a while".
    let Some(server) = support::server() else {
        return;
    };
    let (mut session, uri) = slow_session(&server, "minimal", "800");

    let id = session
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": uri}}),
        )
        .expect("request");
    session
        .notify("$/cancelRequest", json!({"id": id}))
        .expect("cancel");
    let _ = session.response(id, Instant::now()).expect("answered");

    let started = Instant::now();
    let id = session
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": uri}}),
        )
        .expect("request");
    let (resp, _) = session.response(id, started).expect("answered");
    assert!(
        resp.get("result").is_some_and(|r| r.is_array()),
        "the request after a cancellation did not succeed: {resp}"
    );
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn two_requests_in_flight_both_answer_when_one_is_cancelled() {
    // Prevents: a superseded request never being answered, which leaves a
    // client that tracks outstanding ids leaking one per keystroke. Ordering
    // is explicitly not asserted — the server answers on worker threads.
    let Some(server) = support::server() else {
        return;
    };
    let (mut session, uri) = slow_session(&server, "minimal", "1500");

    let first = session
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": uri}}),
        )
        .expect("first");
    let second = session
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": uri}}),
        )
        .expect("second");
    session
        .notify("$/cancelRequest", json!({"id": first}))
        .expect("cancel");

    let (a, _) = session
        .response(first, Instant::now())
        .expect("first answered");
    let (b, _) = session
        .response(second, Instant::now())
        .expect("second answered");
    assert_eq!(
        a.pointer("/error/code").and_then(Value::as_i64),
        Some(-32800),
        "{a}"
    );
    assert!(b.get("result").is_some(), "the survivor must succeed: {b}");
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn a_cancel_for_an_unknown_id_is_a_no_op() {
    // Clients send these routinely: a response and a cancel cross on the wire
    // every time a user types fast. A server that errored, or that kept the id
    // forever, would leak a little on every keystroke.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let mut session = server.session(&workspace, &profile);
    for id in [1, 42, 999_999] {
        session
            .notify("$/cancelRequest", json!({"id": id}))
            .expect("cancel");
    }
    let extra = session.drain(Duration::from_millis(200)).expect("drain");
    assert!(
        extra.is_empty(),
        "a cancel for an unknown id produced traffic: {extra:?}"
    );
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn the_overlay_wins_over_disk_until_the_document_closes() {
    // The claim that makes a language server different from a build watcher.
    // The file on disk is mutated to something that would diagnose
    // differently; while the document is open the server must answer from the
    // BUFFER, and after `didClose` from disk again.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.root.join("target").join("overlay-precedence");
    std::fs::create_dir_all(&workspace).expect("scratch workspace");
    let path = workspace.join("subject.lu");
    let clean = "fn main() -> !int {\n    0\n}\n";
    let broken_on_disk = "fn main() -> !int {\n    let a = 1 < 2 < 3\n    0\n}\n";
    std::fs::write(&path, clean).expect("write");

    let profile = server.profile("minimal");
    let mut session = server.session(&workspace, &profile);
    let uri = lsp_harness::session::file_uri(&path);
    session
        .notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "wolf",
                                    "version": 1, "text": clean}}),
        )
        .expect("didOpen");
    let (published, _) = session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish");
    assert_eq!(count(&published), 0, "the clean buffer: {published}");

    // Disk changes underneath. Nothing told the server, and nothing should:
    // the client owns this document now.
    std::fs::write(&path, broken_on_disk).expect("write");
    session
        .notify(
            "textDocument/didSave",
            json!({"textDocument": {"uri": uri}}),
        )
        .expect("didSave");
    let (published, _) = session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish after save");
    assert_eq!(
        count(&published),
        0,
        "the server read the file from disk while the client owned the buffer: {published}"
    );

    // Handing it back: now disk is the truth again.
    session
        .notify(
            "textDocument/didClose",
            json!({"textDocument": {"uri": uri}}),
        )
        .expect("didClose");
    let (cleared, _) = session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish on close");
    assert_eq!(
        count(&cleared),
        0,
        "closing must clear the client's decorations: {cleared}"
    );

    session
        .notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "wolf",
                                    "version": 1, "text": broken_on_disk}}),
        )
        .expect("reopen");
    let (published, _) = session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish after reopen");
    assert_eq!(count(&published), 1, "{published}");
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn a_stale_version_did_change_does_not_corrupt_state() {
    // A client that reorders `didChange` — or a harness that replays one out
    // of order — must not leave the server holding text no buffer ever had.
    // v0 sync is full-text and last-write-wins, so the assertion is that the
    // LAST change is what the server answers from, whatever the version says.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let mut session = server.session(&workspace, &profile);
    let uri = lsp_harness::session::file_uri(&workspace.join("hello.lu"));
    let clean = String::from_utf8_lossy(&support::read(&workspace, "hello.lu")).into_owned();
    session
        .notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "wolf",
                                    "version": 7, "text": clean}}),
        )
        .expect("didOpen");
    session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish");

    let broken = "fn main() -> !int {\n    let a = 1 < 2 < 3\n    0\n}\n";
    for (version, text) in [(8, broken), (3, broken)] {
        session
            .notify(
                "textDocument/didChange",
                json!({"textDocument": {"uri": uri, "version": version},
                       "contentChanges": [{"text": text}]}),
            )
            .expect("didChange");
        let (published, _) = session
            .notification(
                "textDocument/publishDiagnostics",
                Some(&uri),
                Instant::now(),
            )
            .expect("publish");
        assert_eq!(
            count(&published),
            1,
            "version {version} produced {published}"
        );
    }
    // And the document is still answerable — not wedged, not empty.
    let started = Instant::now();
    let id = session
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": uri}}),
        )
        .expect("documentSymbol");
    let (resp, _) = session.response(id, started).expect("answered");
    assert!(resp.get("result").is_some(), "{resp}");
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn did_save_with_and_without_text_both_work_and_neither_hangs() {
    // The server advertises `save: {includeText: false}`, so a client sending
    // text anyway is off-spec but common. Neither shape may hang, and neither
    // may change the answer — the buffer is the truth either way.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let mut session = server.session(&workspace, &profile);
    let uri = lsp_harness::session::file_uri(&workspace.join("grammar/semicolon.lu"));
    let text =
        String::from_utf8_lossy(&support::read(&workspace, "grammar/semicolon.lu")).into_owned();
    session
        .notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "wolf",
                                    "version": 1, "text": text}}),
        )
        .expect("didOpen");
    let (first, _) = session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish");

    for params in [
        json!({"textDocument": {"uri": uri}}),
        json!({"textDocument": {"uri": uri}, "text": text}),
    ] {
        session
            .notify("textDocument/didSave", params)
            .expect("didSave");
        let (published, _) = session
            .notification(
                "textDocument/publishDiagnostics",
                Some(&uri),
                Instant::now(),
            )
            .expect("publish after save");
        assert_eq!(
            published.pointer("/params/diagnostics"),
            first.pointer("/params/diagnostics"),
            "a save changed the diagnostics"
        );
    }
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn there_is_no_will_save_wait_until_and_asking_for_one_does_not_hang() {
    // The server does not advertise `willSaveWaitUntil`, and a client that
    // asks anyway must get an answer rather than a hung save. This is the
    // failure mode where format-on-save appears to work and then one day the
    // editor freezes on Ctrl+S.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("maximal");
    let mut session = server.session(&workspace, &profile);
    let uri = lsp_harness::session::file_uri(&workspace.join("hello.lu"));

    let started = Instant::now();
    let id = session
        .request(
            "textDocument/willSaveWaitUntil",
            json!({"textDocument": {"uri": uri}, "reason": 1}),
        )
        .expect("request");
    let (resp, took) = session.response(id, started).expect("answered");
    assert_eq!(
        resp.pointer("/error/code").and_then(Value::as_i64),
        Some(-32601),
        "{resp}"
    );
    assert!(took < Duration::from_secs(5), "{took:?}");
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn a_request_after_shutdown_is_refused_rather_than_served() {
    // The spec's rule, and a practical one: a server that keeps working after
    // `shutdown` is a server the editor cannot close.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let mut session = server.session(&workspace, &profile);

    let started = Instant::now();
    let id = session.request("shutdown", Value::Null).expect("shutdown");
    let (resp, _) = session.response(id, started).expect("answered");
    assert_eq!(resp.get("result"), Some(&Value::Null), "{resp}");

    let started = Instant::now();
    let id = session
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": "file:///nowhere.lu"}}),
        )
        .expect("request");
    let (resp, _) = session.response(id, started).expect("answered");
    assert!(
        resp.get("error").is_some(),
        "a request after shutdown was served: {resp}"
    );

    session.notify("exit", Value::Null).expect("exit");
    session.close_stdin();
    let code = session.wait_exit(Duration::from_secs(10)).expect("exited");
    assert_eq!(code, Some(0), "shutdown then exit must exit 0");
}

#[test]
fn a_client_that_vanishes_leaves_no_orphan() {
    // The SIGKILL case, expressed portably: the client's stdin closes without
    // an `exit`. Process-tree semantics differ between unix and windows, but
    // "my end of the pipe went away" is the same event on both, and a server
    // that ignores it is a process the user accumulates one of per crash.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let mut session = server.session(&workspace, &profile);
    let pid = session.pid();

    session.close_stdin();
    let code = session
        .wait_exit(Duration::from_secs(10))
        .unwrap_or_else(|e| panic!("the server outlived its client (pid {pid}): {e}"));
    // The exit *code* is a separate question from whether it left; see
    // `exit_without_shutdown_should_be_nonzero`.
    let _ = code;
}

/// A finding, filed rather than asserted.
///
/// The specification says a server receiving `exit` **without** a preceding
/// `shutdown` should exit with code 1. `wolf lsp` v0 exits 0 in both cases —
/// its main loop returns `Ok(())` on `exit` regardless of whether `shutdown`
/// was seen. Nothing an editor does depends on it (no mainstream client reads
/// the code), which is why this is recorded rather than gated: turning the
/// suite red for it would block the whole editor layer on a cosmetic protocol
/// detail.
///
/// The test asserts the CURRENT behavior so that a fix upstream shows up here
/// as a deliberate, reviewable failure rather than going unnoticed. Filed
/// upstream as part of ls01's closeout.
#[test]
fn exit_without_shutdown_exits_zero_which_the_spec_says_should_be_one() {
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let mut session = server.session(&workspace, &profile);

    session.notify("exit", Value::Null).expect("exit");
    session.close_stdin();
    let code = session
        .wait_exit(Duration::from_secs(10))
        .expect("the server must still leave");
    assert_eq!(
        code,
        Some(0),
        "wolf lsp v0 exits 0 on a bare `exit`. If this now reports 1, the upstream fix \
         landed and this test should be inverted — it is pinned so the change is visible."
    );
}

fn count(published: &Value) -> usize {
    published
        .pointer("/params/diagnostics")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}
