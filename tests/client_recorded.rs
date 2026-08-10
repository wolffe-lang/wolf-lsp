//! Client-recorded transcripts — the rules that make a script-less transcript
//! evidence instead of a golden byte file.
//!
//! Server-free on purpose: these are properties of the committed artifacts, so
//! they hold on a fresh clone with no `wolf` anywhere.

use std::path::PathBuf;

use lsp_harness::profiles::{self, Provenance};
use lsp_transcript::jsonl;
use lsp_transcript::record::Dir;

/// Every `.jsonl` under `transcripts/` with no `.lsps` beside it.
fn script_less() -> Vec<PathBuf> {
    let root = lsp_harness::repo_root();
    let mut found = Vec::new();
    let mut stack = vec![root.join("transcripts")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "jsonl")
                && !path.with_extension("lsps").is_file()
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The exemption `verify` grants is narrow, and this is its premise: a
/// transcript with no script must be one a *named client* recorded, and that
/// client must have a profile read off the client itself. Otherwise the
/// exemption becomes "any transcript nobody wrote a script for".
#[test]
fn a_script_less_transcript_belongs_to_a_real_client() {
    let root = lsp_harness::repo_root();
    let (loaded, errors) = profiles::load_all(&root);
    assert!(errors.is_empty(), "{errors:?}");

    for path in script_less() {
        let text = std::fs::read_to_string(&path).expect("readable");
        let transcript = jsonl::parse(&text).expect("parses");
        let client = transcript
            .header
            .name
            .split('/')
            .next()
            .expect("a name always has a first segment");

        assert!(
            profiles::REAL_CLIENTS.iter().any(|(c, _)| *c == client),
            "{}: names client `{client}`, which this repo does not track — a \
             transcript with no script is only evidence when a named client \
             produced it",
            lsp_harness::slash_path(&path)
        );
        assert!(
            matches!(
                loaded.get(client).map(|p| &p.provenance),
                Some(Provenance::Derived { .. })
            ),
            "{}: `{client}` has no derived profile, so nothing records what \
             this session's client actually declared",
            lsp_harness::slash_path(&path)
        );
        assert_eq!(
            transcript.header.profile,
            client,
            "{}: a client-recorded session must be stamped with that client's \
             own profile",
            lsp_harness::slash_path(&path)
        );
    }
}

/// A capture is only worth committing if it replays, and replay rehydrates
/// exactly one placeholder: `$WS`. A `file://` URI left absolute pins the
/// transcript to the machine that recorded it; a `$URI` placeholder would be
/// *sent* to the server as that literal string.
#[test]
fn captured_client_messages_carry_no_absolute_paths() {
    for path in script_less() {
        let text = std::fs::read_to_string(&path).expect("readable");
        let transcript = jsonl::parse(&text).expect("parses");
        for rec in &transcript.records {
            if rec.dir != Dir::C2s {
                continue;
            }
            let rendered = serde_json::to_string(&rec.params).expect("serializable");
            assert!(
                !rendered.contains("$URI"),
                "{} seq {}: `$URI` is not rehydrated on replay and would be \
                 sent verbatim",
                lsp_harness::slash_path(&path),
                rec.seq
            );
            for marker in ["file:///home/", "file:///Users/", "file:///C:/"] {
                assert!(
                    !rendered.contains(marker),
                    "{} seq {}: an absolute path survived normalization ({marker})",
                    lsp_harness::slash_path(&path),
                    rec.seq
                );
            }
        }
    }
}

/// fackr's session is the first client recording, and the reason it exists is
/// the encoding: the whole patch series turns on this one array reaching the
/// wire. A recording that lost it would still replay green.
#[test]
fn fackr_declared_utf32_on_the_wire() {
    let path = lsp_harness::repo_root()
        .join("transcripts")
        .join("fackr")
        .join("smoke.jsonl");
    if !path.is_file() {
        panic!("the fackr recording is a committed deliverable of ls02");
    }
    let text = std::fs::read_to_string(&path).expect("readable");
    let transcript = jsonl::parse(&text).expect("parses");

    let initialize = transcript
        .records
        .iter()
        .find(|r| r.method.as_deref() == Some("initialize"))
        .expect("a session starts with initialize");
    let offered = &initialize.params.as_ref().expect("params")["capabilities"]["general"]["positionEncodings"];
    assert_eq!(
        offered,
        &serde_json::json!(["utf-32"]),
        "fackr must offer utf-32 alone — this server prefers utf-8, then \
         utf-16, so a fallback in the list is how the fix gets undone"
    );

    let negotiated = transcript
        .records
        .iter()
        .find(|r| r.dir == Dir::S2c && r.id == initialize.id)
        .and_then(|r| r.result.as_ref())
        .map(|r| r["capabilities"]["positionEncoding"].clone())
        .expect("the server answered initialize");
    assert_eq!(negotiated, serde_json::json!("utf-32"));
}
