//! Client-recorded transcripts — the rules that make a script-less transcript
//! evidence instead of a golden byte file.
//!
//! Server-free on purpose: these are properties of the committed artifacts, so
//! they hold on a fresh clone with no `wolf` anywhere. One of them
//! (`no_committed_transcript_carries_an_absolute_path`) reaches every
//! transcript, scripted or not: it started here as the script-less-only check
//! and le06 measured why that was too narrow.

use std::path::PathBuf;

use lsp_harness::profiles::{self, Provenance};
use lsp_transcript::jsonl;
use lsp_transcript::record::Dir;

/// Every `.jsonl` under `transcripts/`, script or no script.
fn every_transcript() -> Vec<PathBuf> {
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
            } else if path.extension().is_some_and(|e| e == "jsonl") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every `.jsonl` under `transcripts/` with no `.lsps` beside it.
fn script_less() -> Vec<PathBuf> {
    every_transcript()
        .into_iter()
        .filter(|p| !p.with_extension("lsps").is_file())
        .collect()
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

/// THE SAME PROPERTY, OVER EVERY TRANSCRIPT AND EVERY FIELD.
///
/// `captured_client_messages_carry_no_absolute_paths` above is the narrow
/// version, and its two narrowings are exactly where the leak lived: it reads
/// only script-less transcripts, and only their `c2s` `params`. The paths that
/// escaped were in **scripted** transcripts, in **`s2c` results**, and — the
/// reason no value-walking normalizer could see them — in object **KEYS**:
/// `WorkspaceEdit.changes` is `{ [uri: DocumentUri]: TextEdit[] }`, so a
/// `rename` or a `codeAction` edit answered to a client that does not declare
/// `documentChanges` stores its URIs nowhere else.
///
/// Measured before the fix, on this branch: eight records across six
/// transcripts carried a developer's home directory. Six cleared by
/// re-recording at the le06 pin with the key-walking normalizer
/// (`encoding/astral-navigate-{utf8,utf16,utf32}`,
/// `navigation/rename-nvim` ×2, `requests/code-action-quickfix`). Two did not,
/// and cannot from here — see `NEEDS_RECAPTURE`.
///
/// Replaying a leaked transcript from a checkout with a different name
/// compares a live `$WS/…` URI against a recorded absolute one and fails on
/// the path, which is the single failure `Stage::Paths` is unconditional to
/// prevent.
#[test]
fn no_committed_transcript_carries_an_absolute_path() {
    // `~/` counts: eglot names its workspace folder through
    // `abbreviate-file-name`, and a tilde pins a transcript to a home
    // directory just as firmly as `/Users/` does.
    const MARKERS: [&str; 6] = [
        "file:///home/",
        "file:///Users/",
        "file:///C:/",
        "/home/",
        "/Users/",
        "~/",
    ];

    /// THE WAIVER, AND IT IS EXHAUSTIVE IN BOTH DIRECTIONS.
    ///
    /// Two client-recorded sessions still carry a path, and neither is a
    /// normalizer gap: both were captured BEFORE the fix, by a real editor,
    /// and a script-less transcript cannot be re-recorded — the whole claim of
    /// the file is that no script decided what the client sent. Clearing them
    /// means running `lspconf capture` against that editor again:
    ///
    ///   * `vscode/smoke.jsonl` seq 39 — a `codeAction` `edit.changes` key,
    ///     the same class the normalizer now walks, captured on a LINUX box
    ///     (`file:///home/…`), so nomad-1 cannot re-capture it at all.
    ///   * `emacs/smoke.jsonl` seq 1 — `workspaceFolders[0].name` in the
    ///     tilde form. `elide_paths` handles that spelling as of le06, so the
    ///     next eglot capture is clean; this file predates it.
    ///
    /// Filed as wolf-lsp#7. A waived file that stops leaking fails this test
    /// too: a waiver nobody retires is a waiver that hides the next leak.
    const NEEDS_RECAPTURE: [&str; 2] = [
        "transcripts/emacs/smoke.jsonl",
        "transcripts/vscode/smoke.jsonl",
    ];

    let root = lsp_harness::repo_root();
    let mut leaks = Vec::new();
    let mut waived_and_leaking = Vec::new();
    for path in every_transcript() {
        let rel = lsp_harness::slash_path(path.strip_prefix(&root).unwrap_or(&path));
        let waived = NEEDS_RECAPTURE.contains(&rel.as_str());
        let text = std::fs::read_to_string(&path).expect("readable");
        let transcript = jsonl::parse(&text).expect("parses");
        let mut leaked_here = false;
        for rec in &transcript.records {
            let rendered = serde_json::to_string(rec).expect("serializable");
            if MARKERS.iter().any(|m| rendered.contains(m)) {
                leaked_here = true;
                if !waived {
                    leaks.push(format!("{rel} seq {}", rec.seq));
                }
            }
        }
        if waived && !leaked_here {
            waived_and_leaking.push(rel);
        }
    }

    assert!(
        leaks.is_empty(),
        "an absolute path survived normalization in {} record(s). Re-record \
         (`lspconf rerecord`) — and if re-recording does not clear it, the \
         path is somewhere the `paths` stage does not walk, which is a \
         normalizer bug and not a transcript one:\n  {}",
        leaks.len(),
        leaks.join("\n  ")
    );
    assert!(
        waived_and_leaking.is_empty(),
        "these are listed in NEEDS_RECAPTURE and no longer leak — delete the \
         entry and close wolf-lsp#7 for them:\n  {}",
        waived_and_leaking.join("\n  ")
    );
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
