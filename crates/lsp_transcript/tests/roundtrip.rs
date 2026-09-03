//! Round-trip and review-pattern tests.
//!
//! Two acceptance items from ls00 live here: a `proptest` proving
//! parse→serialize→parse is a fixed point, and an `insta` snapshot of a
//! hand-written two-message transcript that establishes the review pattern
//! every later snapshot follows.

use lsp_transcript::normalize::Normalizer;
use lsp_transcript::record::{Dir, Header, Kind, Record, Transcript};
use lsp_transcript::{Matcher, Stage, jsonl};
use proptest::prelude::*;
use serde_json::Value;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

// --------------------------------------------------- the review pattern --

#[test]
fn normalized_two_message_transcript() {
    // The snapshot holds the NORMALIZED view (ls01 §3): a diff here is a
    // behavior change, not a different run. Reviewing this diff is the whole
    // ritual — see CONTRIBUTING.md.
    let mut t = jsonl::parse(&fixture("open-hover.jsonl")).unwrap();
    let workspace = std::path::PathBuf::from("/home/dev/wolf-lsp/vendor/upstream/samples");
    Normalizer::new(Some(workspace)).run(&mut t);

    let decisions: Vec<String> = t
        .records
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "seq {} {} {:?} -> {}",
                r.seq,
                r.dir,
                t.method_for(i),
                t.matcher_for(i)
            )
        })
        .collect();

    insta::assert_snapshot!(format!(
        "{}\n{}",
        jsonl::to_string(&t),
        decisions.join("\n")
    ));
}

#[test]
fn normalization_is_idempotent() {
    // Running the pipeline twice must not move anything: a stage that rewrote
    // its own output would make every re-record a diff.
    let mut once = jsonl::parse(&fixture("diagnostics.jsonl")).unwrap();
    let ws = std::path::PathBuf::from("/home/dev/wolf-lsp/vendor/upstream/samples");
    Normalizer::new(Some(ws.clone())).run(&mut once);
    let mut twice = once.clone();
    Normalizer::new(Some(ws)).run(&mut twice);
    assert_eq!(jsonl::to_string(&once), jsonl::to_string(&twice));
}

#[test]
fn two_machines_normalize_to_the_same_transcript() {
    // The same session recorded under different roots and with different pids
    // must compare equal — that is the definition of "incidental".
    let raw = fixture("diagnostics.jsonl");
    let unix_ws = "/home/dev/wolf-lsp/vendor/upstream/samples";
    let win_ws = r"C:\actions\wolf-lsp\vendor\upstream\samples";
    let win_slashed = win_ws.replace('\\', "/");

    let mut a = jsonl::parse(&raw).unwrap();
    Normalizer::new(Some(unix_ws.into())).run(&mut a);

    // The windows twin is built the way a windows RUN builds it, not by a
    // blunt substring swap. A `file:` URI takes three slashes before an
    // absolute path: the unix root supplies the third itself, the drive-letter
    // root does not, and a naive replace produces `file://C:/…` — a URI in
    // which `C:` is the AUTHORITY. Writing the malformed form here would make
    // this test assert that two machines agree about a string neither of them
    // emits. le06 found the real shape on the first windows CI run.
    let win_raw = raw
        .replace(
            &format!("file://{unix_ws}"),
            &format!("file:///{win_slashed}"),
        )
        .replace(unix_ws, &win_ws.replace('\\', "\\\\"));
    let mut b = jsonl::parse(&win_raw).unwrap();
    Normalizer::new(Some(win_ws.into())).run(&mut b);

    assert_eq!(jsonl::to_string(&a), jsonl::to_string(&b));
    assert!(
        jsonl::to_string(&a).contains("$WS"),
        "the workspace was never elided"
    );
}

/// `WorkspaceEdit.changes` is the one LSP map whose KEYS are data.
///
/// A `rename` (or a `codeAction` edit) answered to a client that does not
/// declare `workspaceEdit.documentChanges` — nvim and helix, of the maintained
/// profiles — puts its document URIs in key position and nowhere else. The
/// `paths` stage walked values only, so those URIs shipped **absolute** in six
/// committed transcripts, and any checkout with a different name failed to
/// replay them. le06.
#[test]
fn a_workspace_edit_changes_map_has_its_keys_elided_too() {
    let repo = std::path::PathBuf::from("/home/dev/wolf-lsp");
    let workspace = repo.join("fixtures");

    let mut t = Transcript {
        header: Header {
            transcript: lsp_transcript::FORMAT_VERSION,
            name: "encoding/astral-navigate-utf8".into(),
            wolf_pin: "0".repeat(40),
            profile: "utf8-first".into(),
            workspace: "fixtures".into(),
            recorded: "2026-09-02".into(),
        },
        records: vec![Record {
            seq: 1,
            dir: Dir::S2c,
            kind: Kind::Response,
            id: Some(Value::from(1)),
            method: None,
            params: None,
            result: Some(serde_json::json!({
                "changes": {
                    "file:///home/dev/wolf-lsp/fixtures/astral.lu": [{ "newText": "base" }],
                    "file:///home/dev/wolf-lsp/vendor/upstream/samples/hello.lu": [],
                }
            })),
            error: None,
            matcher: None,
            normalize: Vec::new(),
            t_us: None,
        }],
    };

    Normalizer::new(Some(workspace))
        .with_repo_root(repo)
        .run(&mut t);

    let changes = t.records[0].result.clone().expect("result")["changes"].clone();
    assert!(
        changes.get("file://$WS/astral.lu").is_some(),
        "the workspace key was not elided: {changes}"
    );
    // Two distinct keys stay two: only a prefix is replaced, so nothing can
    // collide and no edit list can be lost.
    assert!(
        changes
            .get("file://$REPO/vendor/upstream/samples/hello.lu")
            .is_some(),
        "the second key was lost or mangled: {changes}"
    );
    assert_eq!(changes.as_object().map(serde_json::Map::len), Some(2));

    let text = jsonl::to_string(&t);
    assert!(
        !text.contains("/home/"),
        "an absolute path survived in a key:\n{text}"
    );
}

/// The tilde form is a real spelling, not a courtesy.
///
/// eglot names its workspace folder through emacs's `abbreviate-file-name`, so
/// `transcripts/emacs/smoke.jsonl` carries `"~/…/wolf-lsp/"` — a home directory
/// in a committed artifact that absolute-prefix matching cannot see, because
/// the absolute prefix is not in the string.
#[test]
fn a_tilde_abbreviated_root_is_elided_too() {
    let Ok(home) = std::env::var("HOME") else {
        return; // no HOME: the stage elides no tilde form, by design.
    };
    let home = home.trim_end_matches('/').to_string();
    let repo = std::path::PathBuf::from(format!("{home}/wolf-lsp"));

    let mut t = Transcript {
        header: Header {
            transcript: lsp_transcript::FORMAT_VERSION,
            name: "emacs/smoke".into(),
            wolf_pin: "0".repeat(40),
            profile: "emacs".into(),
            workspace: "vendor/upstream/samples".into(),
            recorded: "2026-09-02".into(),
        },
        records: vec![Record {
            seq: 1,
            dir: Dir::C2s,
            kind: Kind::Request,
            id: Some(Value::from(1)),
            method: Some("initialize".into()),
            params: Some(serde_json::json!({
                "workspaceFolders": [{ "uri": format!("file://{repo}/", repo = repo.display()),
                                       "name": "~/wolf-lsp/" }],
            })),
            error: None,
            result: None,
            matcher: None,
            normalize: Vec::new(),
            t_us: None,
        }],
    };

    Normalizer::new(Some(repo.join("vendor/upstream/samples")))
        .with_repo_root(repo)
        .run(&mut t);

    let params = t.records[0].params.clone().expect("params");
    assert_eq!(params["workspaceFolders"][0]["name"], "$REPO/");
    assert_eq!(params["workspaceFolders"][0]["uri"], "file://$REPO/");
}

/// A windows URI elides to the SAME placeholder shape a unix one does.
///
/// `file:` needs three slashes before an absolute path. On unix the workspace
/// root supplies the third itself (`/home/dev/…`), so the whole library is
/// written `file://$WS/…`. A windows root is `D:/a/…` with no leading slash,
/// so the live URI is `file:///D:/a/…` and eliding only the plain form leaves
/// `file:///$WS/…` — one slash more than every recorded transcript, and a
/// mismatch on the URI of every `publishDiagnostics`. le06, measured on the
/// first windows `server-lane` run that ever reached a comparison.
#[test]
fn a_windows_drive_uri_elides_to_the_same_placeholder_as_a_unix_one() {
    let unix = jsonl::parse(&format!(
        "{}\n{}\n",
        r#"{"name":"t/uri","profile":"minimal","recorded":"2026-09-02","transcript":1,"wolf_pin":"0000000000000000000000000000000000000000","workspace":"vendor/upstream/samples"}"#,
        r#"{"dir":"s2c","kind":"notification","method":"textDocument/publishDiagnostics","params":{"uri":"file:///home/dev/wolf-lsp/vendor/upstream/samples/hello.lu","diagnostics":[]},"seq":1}"#
    ))
    .unwrap();
    let win = jsonl::parse(&format!(
        "{}\n{}\n",
        r#"{"name":"t/uri","profile":"minimal","recorded":"2026-09-02","transcript":1,"wolf_pin":"0000000000000000000000000000000000000000","workspace":"vendor/upstream/samples"}"#,
        r#"{"dir":"s2c","kind":"notification","method":"textDocument/publishDiagnostics","params":{"uri":"file:///D:/a/wolf-lsp/wolf-lsp/vendor/upstream/samples/hello.lu","diagnostics":[]},"seq":1}"#
    ))
    .unwrap();

    let mut a = unix;
    Normalizer::new(Some("/home/dev/wolf-lsp/vendor/upstream/samples".into())).run(&mut a);
    let mut b = win;
    Normalizer::new(Some(
        r"D:\a\wolf-lsp\wolf-lsp\vendor\upstream\samples".into(),
    ))
    .run(&mut b);

    let uri_a = a.records[0].params.clone().unwrap()["uri"].clone();
    let uri_b = b.records[0].params.clone().unwrap()["uri"].clone();
    assert_eq!(uri_a, serde_json::json!("file://$WS/hello.lu"));
    assert_eq!(
        uri_b, uri_a,
        "a windows recording and a unix one must normalize to one string"
    );
}

#[test]
fn ids_renumber_by_first_appearance_not_by_recorded_value() {
    let text = concat!(
        r#"{"name":"t/ids","profile":"minimal","recorded":"2026-08-09","transcript":1,"#,
        r#""wolf_pin":"ecea37c312595bc7e8fbd20d1240200e1091e234","workspace":"vendor/upstream/samples"}"#,
        "\n",
        r#"{"dir":"c2s","id":900,"kind":"request","method":"initialize","seq":1}"#,
        "\n",
        r#"{"dir":"s2c","id":900,"kind":"response","result":{},"seq":2}"#,
        "\n",
        r#"{"dir":"c2s","id":17,"kind":"request","method":"shutdown","seq":3}"#,
        "\n",
    );
    let mut t = jsonl::parse(text).unwrap();
    Normalizer::new(None).run(&mut t);
    let ids: Vec<&Value> = t.records.iter().filter_map(|r| r.id.as_ref()).collect();
    assert_eq!(ids, vec![&Value::from(1), &Value::from(1), &Value::from(2)]);
}

// ------------------------------------------------------ the fixed point --

fn arb_json(depth: u32) -> BoxedStrategy<Value> {
    // Floats are deliberately absent: they are banned from assertions
    // (report 09 §conformance harness), so generating them would test a shape
    // no transcript may contain.
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::from),
        any::<i32>().prop_map(Value::from),
        "[a-zA-Z0-9/:_.$-]{0,12}".prop_map(Value::from),
    ];
    if depth == 0 {
        return leaf.boxed();
    }
    let inner = arb_json(depth - 1);
    prop_oneof![
        4 => leaf,
        1 => prop::collection::vec(inner.clone(), 0..3).prop_map(Value::from),
        1 => prop::collection::hash_map("[a-z]{1,6}", inner, 0..3)
            .prop_map(|m| Value::Object(m.into_iter().collect())),
    ]
    .boxed()
}

fn arb_matcher() -> impl Strategy<Value = Matcher> {
    prop_oneof![
        Just(Matcher::Exact),
        Just(Matcher::Subset),
        Just(Matcher::Ignore),
        "[a-z]{0,6}".prop_map(|p| Matcher::Set(lsp_transcript::pointer::Pointer::parse(&p))),
        "[a-z]{1,6}".prop_map(|p| Matcher::Regex(lsp_transcript::pointer::Pointer::parse(&p))),
    ]
}

fn arb_record() -> impl Strategy<Value = Record> {
    (
        any::<u32>(),
        prop_oneof![Just(Dir::C2s), Just(Dir::S2c)],
        prop_oneof![
            Just(Kind::Request),
            Just(Kind::Response),
            Just(Kind::Notification)
        ],
        proptest::option::of(any::<i32>().prop_map(Value::from)),
        proptest::option::of("[a-z$/]{1,20}"),
        proptest::option::of(arb_json(2)),
        proptest::option::of(arb_json(2)),
        proptest::option::of(arb_matcher()),
        prop::collection::vec(
            prop_oneof![
                Just(Stage::Ids),
                Just(Stage::Paths),
                Just(Stage::Pid),
                Just(Stage::Uri),
                Just(Stage::Version),
                Just(Stage::ServerInfo),
                Just(Stage::Nulls),
            ],
            0..3,
        ),
        proptest::option::of(any::<u64>()),
    )
        .prop_map(
            |(seq, dir, kind, id, method, params, result, matcher, normalize, t_us)| Record {
                seq,
                dir,
                kind,
                id,
                method,
                params,
                result,
                error: None,
                matcher,
                normalize,
                t_us,
            },
        )
}

fn arb_transcript() -> impl Strategy<Value = Transcript> {
    prop::collection::vec(arb_record(), 0..6).prop_map(|records| Transcript {
        header: Header {
            transcript: lsp_transcript::FORMAT_VERSION,
            name: "prop/generated".to_string(),
            wolf_pin: "ecea37c312595bc7e8fbd20d1240200e1091e234".to_string(),
            profile: "minimal".to_string(),
            workspace: "vendor/upstream/samples".to_string(),
            recorded: "2026-08-09".to_string(),
        },
        records,
    })
}

proptest! {
    /// parse ∘ serialize is the identity on serialized transcripts.
    ///
    /// This is what makes a re-record reviewable: if serialization were not a
    /// fixed point, every re-record would carry churn that hides the one line
    /// that actually changed.
    #[test]
    fn serialize_parse_serialize_is_a_fixed_point(t in arb_transcript()) {
        let once = jsonl::to_string(&t);
        let reparsed = jsonl::parse(&once).expect("our own output must parse");
        let twice = jsonl::to_string(&reparsed);
        prop_assert_eq!(&once, &twice);
        prop_assert_eq!(&reparsed, &jsonl::parse(&twice).unwrap());
        prop_assert!(once.ends_with('\n'));
        prop_assert!(!once.contains('\r'), "canonical form is LF only");
    }

    /// Every line is exactly one record: no embedded newlines, ever.
    #[test]
    fn one_message_per_line(t in arb_transcript()) {
        let text = jsonl::to_string(&t);
        prop_assert_eq!(text.lines().count(), t.records.len() + 1);
    }

    /// Object keys come out sorted at every depth.
    #[test]
    fn keys_are_sorted_everywhere(v in arb_json(3)) {
        let mut sorted = v.clone();
        lsp_transcript::record::sort_keys(&mut sorted);
        fn check(v: &Value) -> bool {
            match v {
                Value::Object(m) => {
                    let keys: Vec<&String> = m.keys().collect();
                    let mut want = keys.clone();
                    want.sort();
                    keys == want && m.values().all(check)
                }
                Value::Array(a) => a.iter().all(check),
                _ => true,
            }
        }
        prop_assert!(check(&sorted));
    }
}

// ------------------------------------------------- the repository root --

/// A client may pick a root ABOVE the workspace, and the transcript must still
/// be machine-independent.
///
/// This is a regression test with a date on it: helix (`roots = ["wolf.pkg",
/// ".git"]`) and eglot (project.el, which finds the git root) both send
/// `rootUri` = the repository while every document URI is under
/// `vendor/upstream/samples`. Before ls06 the normalizer elided only the
/// workspace, so the recording machine's home directory survived into two
/// committed transcripts —
/// `client_recorded::captured_client_messages_carry_no_absolute_paths` caught
/// it, which is exactly what that test is for.
#[test]
fn a_root_above_the_workspace_is_elided_to_repo() {
    let repo = std::path::PathBuf::from("/home/dev/wolf-lsp");
    let workspace = repo.join("vendor/upstream/samples");

    let mut t = Transcript {
        header: Header {
            transcript: lsp_transcript::FORMAT_VERSION,
            name: "helix/smoke".into(),
            wolf_pin: "0".repeat(40),
            profile: "helix".into(),
            workspace: "vendor/upstream/samples".into(),
            recorded: "2026-08-10".into(),
        },
        records: vec![Record {
            seq: 1,
            dir: Dir::C2s,
            kind: Kind::Request,
            id: Some(Value::from(1)),
            method: Some("initialize".into()),
            params: Some(serde_json::json!({
                "rootPath": "/home/dev/wolf-lsp",
                "rootUri": "file:///home/dev/wolf-lsp",
                "workspaceFolders": [{ "uri": "file:///home/dev/wolf-lsp", "name": "wolf-lsp" }],
                "doc": "file:///home/dev/wolf-lsp/vendor/upstream/samples/hello.lu",
            })),
            result: None,
            error: None,
            matcher: None,
            normalize: Vec::new(),
            t_us: None,
        }],
    };

    Normalizer::new(Some(workspace))
        .with_repo_root(repo)
        .run(&mut t);

    let params = t.records[0].params.clone().expect("params");
    assert_eq!(params["rootPath"], "$REPO");
    assert_eq!(params["rootUri"], "file://$REPO");
    assert_eq!(params["workspaceFolders"][0]["uri"], "file://$REPO");
    // Order matters: the workspace is the DEEPER path and must win, or every
    // document URI in every existing transcript changes shape.
    assert_eq!(params["doc"], "file://$WS/hello.lu");

    let text = jsonl::to_string(&t);
    assert!(
        !text.contains("/home/"),
        "an absolute path survived normalization:\n{text}"
    );
}
