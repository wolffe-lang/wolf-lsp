//! The matcher engine, against hand-written fixture transcripts.
//!
//! No server is involved and none is needed: a matcher is a pure function of
//! two JSON values, and ls00's job is to make the *comparison policy* testable
//! before there is anything to compare (D34 — `wolf lsp` ships with wolf-lang's
//! s52). Every test here is a claim about what the suite will and will not let
//! through once ls01 wires it to a live process.

use lsp_transcript::matcher::Matcher;
use lsp_transcript::pointer::Pointer;
use lsp_transcript::{Dir, Kind, jsonl};
use serde_json::json;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

// --------------------------------------------------------------- parsing --

#[test]
fn the_fixtures_parse_and_validate() {
    for name in ["open-hover.jsonl", "diagnostics.jsonl"] {
        let t = jsonl::parse(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        t.validate()
            .unwrap_or_else(|errs| panic!("{name}: {errs:#?}"));
    }
}

#[test]
fn the_fixtures_are_already_canonical() {
    // Hand-written files are the review pattern; if serializing them changes
    // anything, the pattern is a lie and every future re-record diff is noise.
    for name in ["open-hover.jsonl", "diagnostics.jsonl"] {
        let text = fixture(name);
        let t = jsonl::parse(&text).unwrap();
        assert_eq!(jsonl::to_string(&t), text, "{name} is not canonical");
    }
}

#[test]
fn a_bad_line_is_reported_with_its_line_number() {
    let text = format!("{}{{ not json\n", fixture("open-hover.jsonl"));
    let err = jsonl::parse(&text).unwrap_err();
    assert_eq!(err.line, 4, "{err}");
}

#[test]
fn an_unknown_matcher_name_is_refused_at_parse_time() {
    let bad = format!(
        "{}{}\n",
        fixture("open-hover.jsonl"),
        r#"{"dir":"s2c","id":9,"kind":"response","match":"approximately","result":{},"seq":3}"#
    );
    let err = jsonl::parse(&bad).unwrap_err();
    assert!(err.message.contains("unknown matcher"), "{err}");
}

#[test]
fn an_unknown_normalization_stage_is_refused_at_parse_time() {
    let bad = format!(
        "{}{}\n",
        fixture("open-hover.jsonl"),
        r#"{"dir":"s2c","id":9,"kind":"response","normalize":["obliterate"],"result":{},"seq":3}"#
    );
    let err = jsonl::parse(&bad).unwrap_err();
    assert!(err.message.contains("unknown normalization stage"), "{err}");
}

// -------------------------------------------------------------- defaults --

#[test]
fn a_response_inherits_the_method_of_its_request() {
    // The response line carries no method; the default table still has to
    // reach `initialize`, or the single most important default never applies.
    let t = jsonl::parse(&fixture("open-hover.jsonl")).unwrap();
    assert_eq!(t.method_for(1), Some("initialize"));
    assert_eq!(t.matcher_for(1), Matcher::Subset);
}

#[test]
fn server_capabilities_default_to_subset() {
    // A server that GAINS a capability must not turn the suite red...
    let recorded = json!({"capabilities": {"hoverProvider": true}});
    let grown = json!({"capabilities": {"hoverProvider": true, "renameProvider": true}});
    assert!(Matcher::Subset.compare(&recorded, &grown).is_ok());

    // ...and one that LOSES a capability the transcript relied on must.
    let shrunk = json!({"capabilities": {"renameProvider": true}});
    let err = Matcher::Subset.compare(&recorded, &shrunk).unwrap_err();
    assert_eq!(err.path, "/capabilities/hoverProvider");
}

#[test]
fn publish_diagnostics_defaults_to_a_multiset_over_diagnostics() {
    let t = jsonl::parse(&fixture("diagnostics.jsonl")).unwrap();
    assert_eq!(
        t.matcher_for(1),
        Matcher::Set(Pointer::parse("diagnostics"))
    );
}

#[test]
fn incidental_chatter_is_ignored_and_formatting_is_exact() {
    use lsp_transcript::defaults::for_method;
    assert_eq!(
        for_method(Some("window/logMessage"), Kind::Notification),
        Matcher::Ignore
    );
    assert_eq!(
        for_method(Some("textDocument/formatting"), Kind::Response),
        Matcher::Exact
    );
    assert_eq!(
        for_method(Some("textDocument/documentSymbol"), Kind::Response),
        Matcher::Set(Pointer::parse(""))
    );
}

// --------------------------------------------------------------- subset --

#[test]
fn subset_treats_a_recorded_null_as_satisfied_by_an_absent_key() {
    // LSP's own optionality: clients disagree about sending `null` vs omitting.
    let recorded = json!({"range": null, "contents": "x"});
    let live = json!({"contents": "x"});
    assert!(Matcher::Subset.compare(&recorded, &live).is_ok());
}

#[test]
fn subset_does_not_silently_accept_a_reordered_array() {
    // Order-insensitivity is `set:`'s job, and opting into it must be visible
    // in the transcript rather than inherited from the containment matcher.
    let recorded = json!([1, 2]);
    let live = json!([2, 1]);
    assert!(Matcher::Subset.compare(&recorded, &live).is_err());
}

// ---------------------------------------------------------------- exact --

#[test]
fn exact_rejects_an_added_field_where_subset_accepts_it() {
    let recorded = json!({"a": 1});
    let live = json!({"a": 1, "b": 2});
    assert!(Matcher::Subset.compare(&recorded, &live).is_ok());
    let err = Matcher::Exact.compare(&recorded, &live).unwrap_err();
    assert_eq!(err.path, "/b");
}

#[test]
fn exact_locates_the_deepest_difference() {
    let recorded = json!({"range": {"start": {"line": 4, "character": 4}}});
    let live = json!({"range": {"start": {"line": 4, "character": 5}}});
    let err = Matcher::Exact.compare(&recorded, &live).unwrap_err();
    assert_eq!(err.path, "/range/start/character");
    assert!(err.reason.contains("expected 4, got 5"), "{err}");
}

// ------------------------------------------------------------------ set --

#[test]
fn set_accepts_reordering_but_not_a_changed_span() {
    let t = jsonl::parse(&fixture("diagnostics.jsonl")).unwrap();
    let matcher = t.matcher_for(1);
    let recorded = t.records[1].payload().clone();

    // Same two diagnostics, emitted in the other order: green. Ordering is
    // what LSP leaves free.
    let mut swapped = recorded.clone();
    swapped["diagnostics"].as_array_mut().unwrap().swap(0, 1);
    matcher
        .compare(&recorded, &swapped)
        .expect("reordering diagnostics must not fail");

    // One span moved by a single column: red. Spans are behavior — this is the
    // deliberate-red half of ls01's stability demonstration.
    let mut moved = recorded.clone();
    moved["diagnostics"][0]["range"]["start"]["character"] = json!(5);
    assert!(matcher.compare(&recorded, &moved).is_err());
}

#[test]
fn set_counts_duplicates() {
    let matcher = Matcher::Set(Pointer::parse("d"));
    let recorded = json!({"d": [1, 1, 2]});
    let live = json!({"d": [1, 2, 2]});
    assert!(
        matcher.compare(&recorded, &live).is_err(),
        "multiset, not set"
    );
}

#[test]
fn set_still_checks_everything_outside_the_array() {
    let matcher = Matcher::Set(Pointer::parse("diagnostics"));
    let recorded = json!({"uri": "file://$WS/hello.lu", "diagnostics": []});
    let elsewhere = json!({"uri": "file://$WS/other.lu", "diagnostics": []});
    let err = matcher.compare(&recorded, &elsewhere).unwrap_err();
    assert_eq!(err.path, "/uri");
}

#[test]
fn set_at_the_payload_root_handles_a_bare_array_result() {
    // `textDocument/documentSymbol` returns the array itself.
    let matcher = Matcher::Set(Pointer::parse(""));
    let recorded = json!([{"name": "main"}, {"name": "helper"}]);
    let live = json!([{"name": "helper"}, {"name": "main"}]);
    assert!(matcher.compare(&recorded, &live).is_ok());
    let missing = json!([{"name": "helper"}, {"name": "gone"}]);
    assert!(matcher.compare(&recorded, &missing).is_err());
}

#[test]
fn set_reports_a_length_change_rather_than_hunting_for_a_pair() {
    let matcher = Matcher::Set(Pointer::parse("d"));
    let err = matcher
        .compare(&json!({"d": [1, 2]}), &json!({"d": [1]}))
        .unwrap_err();
    assert!(
        err.reason
            .contains("2 element(s) in the transcript, 1 live"),
        "{err}"
    );
}

// ---------------------------------------------------------------- regex --

#[test]
fn regex_matches_prose_without_pinning_its_wording() {
    // D22 owns diagnostic wording upstream; this repo must not become a second
    // review gate that blocks a message improvement.
    let matcher = Matcher::Regex(Pointer::parse("message"));
    let recorded = json!({"code": "E0501", "message": "^the trait bound"});
    let reworded = json!({"code": "E0501", "message": "the trait bound is not satisfied here"});
    assert!(matcher.compare(&recorded, &reworded).is_ok());

    // The code beside it is still compared.
    let recoded = json!({"code": "E9999", "message": "the trait bound is not satisfied"});
    assert_eq!(
        matcher.compare(&recorded, &recoded).unwrap_err().path,
        "/code"
    );
}

#[test]
fn an_invalid_regex_is_a_located_failure_not_a_panic() {
    let matcher = Matcher::Regex(Pointer::parse("m"));
    let err = matcher
        .compare(&json!({"m": "("}), &json!({"m": "x"}))
        .unwrap_err();
    assert!(err.reason.contains("invalid regex"), "{err}");
}

// --------------------------------------------------------------- ignore --

#[test]
fn ignore_matches_anything_including_absent_payloads() {
    assert!(
        Matcher::Ignore
            .compare(&json!({"a": 1}), &json!(null))
            .is_ok()
    );
}

// ------------------------------------------------------ matcher grammar --

#[test]
fn matcher_strings_round_trip_through_their_written_form() {
    for s in [
        "exact",
        "subset",
        "ignore",
        "set:diagnostics",
        "set:",
        "regex:/message",
    ] {
        let m: Matcher = s.parse().unwrap();
        assert_eq!(m.to_string(), s, "{s} did not round-trip");
    }
}

#[test]
fn a_leading_slash_is_optional_and_escapes_are_rfc6901() {
    assert_eq!(
        Pointer::parse("a/b").tokens(),
        Pointer::parse("/a/b").tokens()
    );
    assert!(Pointer::parse("").is_root() && Pointer::parse("/").is_root());
    assert_eq!(Pointer::parse("a~1b").tokens(), ["a/b"]);
    assert_eq!(Pointer::parse("a~01").tokens(), ["a~1"]);
}

// ------------------------------------------------------------- envelope --

#[test]
fn validate_catches_the_envelope_mistakes_a_recorder_can_make() {
    let mut t = jsonl::parse(&fixture("open-hover.jsonl")).unwrap();
    t.records[0].seq = 7;
    t.records[1].id = None;
    t.records[0].dir = Dir::C2s;
    t.records[0].matcher = Some(Matcher::Exact);
    let errs = t.validate().unwrap_err();
    assert!(errs.iter().any(|e| e.contains("seq is 7")), "{errs:#?}");
    assert!(
        errs.iter().any(|e| e.contains("response without id")),
        "{errs:#?}"
    );
    assert!(
        errs.iter().any(|e| e.contains("c2s records are sent")),
        "{errs:#?}"
    );
}

#[test]
fn a_header_with_an_abbreviated_pin_is_rejected() {
    let mut t = jsonl::parse(&fixture("open-hover.jsonl")).unwrap();
    t.header.wolf_pin = "ecea37c".to_string();
    let errs = t.validate().unwrap_err();
    assert!(
        errs.iter().any(|e| e.contains("40-char hex sha")),
        "{errs:#?}"
    );
}

/// An error response must not be compared with the *method's* matcher.
///
/// Regression from ls01's first full replay: `textDocument/documentSymbol`
/// defaults to `set:` (its result is an array), and a `$/cancelRequest` turns
/// that same exchange into `{"code":-32800,"message":"…"}`. Routing the error
/// through `set:` reported "needs an array in the transcript" while printing
/// two identical payloads — a mismatch about a difference that was not there.
#[test]
fn an_error_response_defaults_to_subset_whatever_the_method() {
    use lsp_transcript::record::{Dir, Kind, Record};

    let error = Record {
        seq: 1,
        dir: Dir::S2c,
        kind: Kind::Response,
        id: Some(serde_json::json!(2)),
        method: None,
        params: None,
        result: None,
        error: Some(serde_json::json!({"code": -32800, "message": "cancelled"})),
        matcher: None,
        normalize: Vec::new(),
        t_us: None,
    };
    assert_eq!(
        error.effective_matcher(Some("textDocument/documentSymbol")),
        Matcher::Subset
    );
    // The success case still gets the method's matcher.
    let ok = Record {
        result: Some(serde_json::json!([])),
        error: None,
        ..error.clone()
    };
    assert!(matches!(
        ok.effective_matcher(Some("textDocument/documentSymbol")),
        Matcher::Set(_)
    ));
    // And an explicit `match` still wins over both.
    let pinned = Record {
        matcher: Some(Matcher::Exact),
        ..error.clone()
    };
    assert_eq!(
        pinned.effective_matcher(Some("textDocument/documentSymbol")),
        Matcher::Exact
    );
}

/// The two payloads that fooled the old default, compared for real.
#[test]
fn two_identical_error_payloads_compare_equal() {
    let payload = serde_json::json!({"code": -32601, "message": "no such method"});
    assert!(Matcher::Subset.compare(&payload, &payload).is_ok());
}
