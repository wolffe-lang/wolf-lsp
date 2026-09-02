//! §4 — capability negotiation, asserted as *properties* rather than as a
//! golden blob.
//!
//! A snapshot of `ServerCapabilities` per profile would go stale the first time
//! the server gained anything, and the reviewer approving that diff would have
//! no way to tell a legitimate addition from a regression. So the transcripts
//! hold the snapshot (`transcripts/lifecycle/*.jsonl`, compared `subset`) and
//! this file holds the claims a snapshot cannot make: that the negotiated
//! encoding is one the client offered, that the server never advertises
//! something it cannot deliver to this client, that `minimal` still gets
//! diagnostics, and that an unknown method answers rather than hangs.

mod support;

use std::time::{Duration, Instant};

use lsp_transcript::Encoding;
use serde_json::{Value, json};

#[test]
fn every_profile_negotiates_the_encoding_it_declares() {
    let Some(server) = support::server() else {
        return;
    };
    let (loaded, errors) = lsp_harness::profiles::load_all(&server.root);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(!loaded.is_empty(), "no profiles to validate");

    for (name, profile) in &loaded {
        let mut session = server.session(&server.samples(), profile);
        assert_eq!(
            session.encoding(),
            profile.expects_encoding,
            "profile `{name}` declares it expects {} but negotiated {}",
            profile.expects_encoding,
            session.encoding()
        );

        // The protocol's own rule: the answer must be one the client offered,
        // or the utf-16 default when it offered nothing usable. A server that
        // echoed back an encoding the client never named would corrupt every
        // position in the session, silently.
        let offered = profile.offered_encodings();
        assert!(
            offered.contains(&session.encoding()) || session.encoding() == Encoding::Utf16,
            "profile `{name}` offered {offered:?} and got {}",
            session.encoding()
        );
        session.shutdown_exit().expect("clean shutdown");
    }
}

#[test]
fn the_server_never_advertises_what_this_client_cannot_receive() {
    // The classic trap §4 names: a capability whose *delivery* needs a client
    // feature the profile did not declare. Dynamic registration is the usual
    // one — a server that advertises `{"id": …}` registration options to a
    // client with no `dynamicRegistration` has advertised something it can
    // never actually register.
    let Some(server) = support::server() else {
        return;
    };
    let profile = server.profile("minimal");
    assert!(
        !profile.declares_dynamic_registration(),
        "the `minimal` profile is supposed to declare nothing optional"
    );
    let mut session = server.session(&server.samples(), &profile);

    // v0 advertises static capabilities only, so every provider must be a
    // plain boolean or an options object with no `id`.
    let caps = capabilities(&server);
    for (key, value) in caps.as_object().expect("capabilities is an object") {
        if let Some(id) = value.get("id") {
            panic!(
                "`{key}` is advertised with a registration id ({id}) to a client that \
                 never declared dynamicRegistration — the server cannot deliver it"
            );
        }
    }
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn the_server_sends_no_client_bound_requests_at_all() {
    // Report 09's hygiene list: facsimile never answers a server→client
    // request, so one sent before `initialized` — or ever, without a timeout —
    // wedges the editor. v0's answer is to send none, and this is what makes
    // that a checked property rather than a claim in a doc comment.
    let Some(server) = support::server() else {
        return;
    };
    let profile = server.profile("maximal");
    let workspace = server.samples();
    let mut session = server.session(&workspace, &profile);

    let text = String::from_utf8_lossy(&support::read(&workspace, "errors.lu")).into_owned();
    let uri = lsp_harness::session::file_uri(&workspace.join("errors.lu"));
    session
        .notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "wolf",
                                    "version": 1, "text": text}}),
        )
        .expect("didOpen");
    let started = Instant::now();
    session
        .notification("textDocument/publishDiagnostics", Some(&uri), started)
        .expect("first publish");
    let extra = session.drain(Duration::from_millis(300)).expect("drain");

    for msg in &extra {
        assert!(
            !(msg.get("id").is_some() && msg.get("method").is_some()),
            "the server sent a client-bound request: {msg}"
        );
    }
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn minimal_still_gets_diagnostics() {
    // The floor: a client that declared nothing optional is still a client,
    // and diagnostics are not an optional feature — they are the reason the
    // server exists. A profile-gated diagnostics path would make the poorest
    // client the one with no errors shown.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let mut session = server.session(&workspace, &profile);

    let name = "traits/golden_missing_bound.lu";
    let text = String::from_utf8_lossy(&support::read(&workspace, name)).into_owned();
    let uri = lsp_harness::session::file_uri(&workspace.join(name));
    session
        .notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "wolf",
                                    "version": 1, "text": text}}),
        )
        .expect("didOpen");
    let (published, _) = session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish");
    let diagnostics = published
        .pointer("/params/diagnostics")
        .and_then(Value::as_array)
        .expect("a diagnostics array");
    assert_eq!(
        diagnostics.len(),
        1,
        "the corpus pins this file at fail(E0501): {published}"
    );
    assert_eq!(diagnostics[0]["code"], "E0501");
    // Related information is *not* gated on the profile declaring support for
    // it either. A client that cannot render it ignores the member; a server
    // that withheld it would make the poorest client also the least informed.
    assert!(
        diagnostics[0].get("relatedInformation").is_some(),
        "E0501 carries two secondary spans: {}",
        diagnostics[0]
    );
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn an_unknown_method_answers_method_not_found_and_never_hangs() {
    let Some(server) = support::server() else {
        return;
    };
    let profile = server.profile("minimal");
    let mut session = server.session(&server.samples(), &profile);

    // Three shapes: a real LSP method this server does not implement, a `$/`
    // method (which the spec singles out), and something entirely invented.
    //
    // `textDocument/rename` USED to head this list and no longer can: wolf-lang
    // s133 implemented it (with `prepareRename`), so it answers a result rather
    // than `-32601`, and asserting the refusal here made this test fail against
    // any binary that actually serves the capability the MATRIX credits it
    // with. Caught by le05's gauntlet at the v0.2.2 re-pin; it was already red
    // at trunk's own pin, so this is a stale test being corrected, not a
    // regression being papered over. `signatureHelp` took its place as the
    // real-but-unimplemented shape — and wolf-lang s134 served THAT too,
    // with `semanticTokens/full` beside it, so this list moved again on the
    // `s134-transcripts` branch: `typeDefinition` and the semantic-token
    // DELTA are the "not served" row now (docs/MATRIX.md), and the positive
    // half is asserted below. A probe of a served method proves nothing.
    for method in [
        "textDocument/typeDefinition",
        "textDocument/semanticTokens/full/delta",
        "$/somethingElse",
        "wolf/notARealExtension",
    ] {
        let started = Instant::now();
        let id = session
            .request(method, json!({}))
            .unwrap_or_else(|e| panic!("send {method}: {e}"));
        let (resp, took) = session
            .response(id, started)
            .unwrap_or_else(|e| panic!("{method} was never answered: {e}"));
        assert_eq!(
            resp.pointer("/error/code").and_then(Value::as_i64),
            Some(-32601),
            "{method} answered {resp}"
        );
        assert!(
            took < Duration::from_secs(5),
            "{method} took {took:?} to say it does not exist"
        );
    }

    // The other half of the same claim: a method the server DOES implement must
    // not answer `-32601`. Without this, moving a method off the list above
    // could silently mean "we stopped testing it" rather than "it is served".
    for method in [
        "textDocument/rename",
        "textDocument/definition",
        "textDocument/references",
    ] {
        let started = Instant::now();
        let id = session
            .request(method, json!({}))
            .unwrap_or_else(|e| panic!("send {method}: {e}"));
        let (resp, _) = session
            .response(id, started)
            .unwrap_or_else(|e| panic!("{method} was never answered: {e}"));
        assert_ne!(
            resp.pointer("/error/code").and_then(Value::as_i64),
            Some(-32601),
            "{method} is served since wolf-lang s133 and must not answer \
             MethodNotFound: {resp}"
        );
    }
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn the_negotiated_encoding_holds_for_the_whole_session() {
    // A server that renegotiated mid-session — or that used one encoding for
    // diagnostics and another for hover — would produce a buffer that drifts
    // as the user edits. The same token is probed twice, far apart in the
    // session and across an edit, and must answer at the same column both
    // times.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.fixtures();
    let profile = server.profile("utf32-only");
    let mut session = server.session(&workspace, &profile);
    assert_eq!(session.encoding(), Encoding::Utf32);

    let bytes = support::read(&workspace, "astral.lu");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let uri = lsp_harness::session::file_uri(&workspace.join("astral.lu"));
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

    let probe = |session: &mut lsp_harness::Session| -> Value {
        let started = Instant::now();
        let id = session
            .request(
                "textDocument/hover",
                json!({"textDocument": {"uri": uri},
                       "position": {"line": 24, "character": 8}}),
            )
            .expect("hover");
        session.response(id, started).expect("hover answered").0
    };

    let first = probe(&mut session);
    session
        .notify(
            "textDocument/didChange",
            json!({"textDocument": {"uri": uri, "version": 2},
                   "contentChanges": [{"text": text}]}),
        )
        .expect("didChange");
    session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish after edit");
    let second = probe(&mut session);

    assert_eq!(
        first.pointer("/result/range"),
        second.pointer("/result/range"),
        "the same token answered at different positions within one session"
    );
    session.shutdown_exit().expect("clean shutdown");
}

/// The `initialize` capabilities of an already-initialized session.
///
/// Re-handshakes on a second connection rather than caching, because
/// `Session::initialize` deliberately returns the whole response and the test
/// that wants only the capabilities should not have to keep it.
fn capabilities(server: &support::Server) -> Value {
    let profile = server.profile("minimal");
    let mut probe = lsp_harness::Session::spawn(&server.bin, &server.samples()).expect("spawn");
    let result = probe.initialize(&profile.capabilities).expect("initialize");
    let caps = result
        .pointer("/result/capabilities")
        .cloned()
        .expect("initialize result carries capabilities");
    probe.shutdown_exit().expect("clean shutdown");
    caps
}
