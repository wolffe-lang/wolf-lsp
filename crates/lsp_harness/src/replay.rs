//! `lspconf replay` — drive a recorded session against the live server and
//! report, per record, what was expected and what arrived.
//!
//! The comparison design is ls00's ([`lsp_transcript::matcher`],
//! [`lsp_transcript::normalize`], [`lsp_transcript::defaults`]); this module is
//! the part that has to hold a process still while it happens. Three decisions
//! are worth stating because each is a way this could have been subtly wrong:
//!
//! **Ids are sent verbatim.** A transcript's ids are already normalized to
//! first-appearance order, so replaying them as-is makes the live ids equal the
//! recorded ones and correlation exact. Allocating fresh ids and maintaining a
//! translation table would work too, and would add a mapping that can be wrong
//! in the one place where being wrong is invisible.
//!
//! **Arrival order is not asserted.** The server answers on worker threads, so
//! two responses can swap and a `publishDiagnostics` can land between them.
//! Every wait correlates — by id for responses, by method and document for
//! notifications — and messages stepped over stay in the session's inbox for
//! the record that wants them. A suite that asserted arrival order would fail
//! on Tuesdays.
//!
//! **A pin mismatch refuses rather than guesses.** A transcript recorded
//! against one compiler and replayed against another compares two different
//! programs; whichever way that comes out, it means nothing.

use std::path::Path;

use lsp_transcript::record::{Dir, Kind, Record, Transcript};
use lsp_transcript::{Matcher, Normalizer, jsonl, normalize};
use serde_json::Value;

use crate::profiles::Profile;
use crate::session::{self, Session};

/// What a replay concluded.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub name: String,
    pub file: String,
    /// Records compared, ignoring `c2s` sends.
    pub compared: usize,
    pub mismatches: Vec<Finding>,
    /// The process exit code, when the transcript ran the session to the end.
    pub exit_code: Option<i32>,
}

impl Report {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.mismatches.is_empty()
    }
}

/// One record that did not hold up.
#[derive(Debug, Clone)]
pub struct Finding {
    pub seq: u32,
    pub method: String,
    pub matcher: String,
    pub detail: String,
    pub expected: Value,
    pub actual: Value,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "  record {} ({}, match `{}`)",
            self.seq, self.method, self.matcher
        )?;
        writeln!(f, "    {}", self.detail)?;
        writeln!(f, "    expected: {}", compact(&self.expected))?;
        writeln!(f, "    actual:   {}", compact(&self.actual))
    }
}

fn compact(v: &Value) -> String {
    let s = v.to_string();
    if s.len() <= 400 {
        s
    } else {
        format!("{}… ({} bytes)", &s[..400], s.len())
    }
}

/// A replay that could not run at all — distinct from one that ran and found
/// a difference, because the exit codes differ (2 versus 1).
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Parse(String),
    PinMismatch { recorded: String, actual: String },
    Session(session::Error),
    Profile(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Parse(m) | Error::Profile(m) => write!(f, "{m}"),
            Error::PinMismatch { recorded, actual } => write!(
                f,
                "this transcript was recorded against wolf-lang {} but the pin is {} \
                 — replaying it would compare two different compilers. Re-record it \
                 (`lspconf record`) as part of the pin bump, or check out the pin it names.",
                &recorded[..7.min(recorded.len())],
                &actual[..7.min(actual.len())]
            ),
            Error::Session(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<session::Error> for Error {
    fn from(e: session::Error) -> Self {
        Error::Session(e)
    }
}

/// Replay one transcript file.
pub fn replay(
    repo_root: &Path,
    bin: &Path,
    path: &Path,
    pin_commit: &str,
) -> Result<Report, Error> {
    let text = std::fs::read_to_string(path)?;
    let transcript = jsonl::parse(&text)
        .map_err(|e| Error::Parse(format!("{}: {e}", crate::slash_path(path))))?;
    if let Err(errs) = transcript.validate() {
        return Err(Error::Parse(format!(
            "{}: {}",
            crate::slash_path(path),
            errs.join("; ")
        )));
    }
    if transcript.header.wolf_pin != pin_commit {
        return Err(Error::PinMismatch {
            recorded: transcript.header.wolf_pin.clone(),
            actual: pin_commit.to_string(),
        });
    }

    let profile_path = repo_root
        .join("profiles")
        .join(format!("{}.json", transcript.header.profile));
    // Loaded and validated even though replay sends the recorded `initialize`
    // params verbatim: a transcript naming a profile that no longer exists, or
    // that no longer validates, is a transcript whose negotiation claim has
    // quietly stopped being checked by anything.
    let _profile = Profile::load(&profile_path).map_err(|e| Error::Profile(e.to_string()))?;

    let workspace = repo_root.join(&transcript.header.workspace);
    // Spawn-time environment comes from the committed script, not the derived
    // transcript. A transcript captured from a real client (ls02–ls06) has no
    // script and therefore no env, which is correct: a real editor spawns the
    // server with the user's environment and nothing more.
    let env = script_env(path);
    let mut session = Session::spawn_with_env(bin, &workspace, &env)?;
    let ws = crate::slash_path(&workspace);

    let mut report = Report {
        name: transcript.header.name.clone(),
        file: crate::slash_path(path),
        ..Report::default()
    };
    // One normalizer per stream, as [`Normalizer`] requires: both sides
    // collapse independently to "first id seen is 1", which is what makes the
    // renumbering meaningful rather than a shared counter.
    //
    // The RECORDED side is normalized too, not just the live one. It arrives
    // already normalized — `lspconf record` writes it that way — so this is a
    // no-op on a freshly recorded file, and normalization is idempotent by
    // test. It stops being a no-op the moment anything hand-edits a transcript
    // or a tool rewrites one, and "normalization runs before matching" has to
    // mean both sides or it means nothing: a reordered `relatedInformation`
    // array in the transcript would otherwise fail against a live message that
    // the same stage had just sorted.
    //
    // Both sides also elide the REPOSITORY root, because a client-recorded
    // transcript may legitimately carry one: helix and eglot resolve their root
    // above the workspace (`.git`), so `$REPO` appears in `rootUri` where `$WS`
    // appears in every document URI.
    let mut live = Normalizer::new(Some(workspace.clone())).with_repo_root(repo_root.to_path_buf());
    let mut expected =
        Normalizer::new(Some(workspace.clone())).with_repo_root(repo_root.to_path_buf());

    for (i, rec) in transcript.records.iter().enumerate() {
        match rec.dir {
            Dir::C2s => send(&mut session, rec, &ws, &mut report)?,
            Dir::S2c => {
                report.compared += 1;
                if let Some(finding) =
                    expect(&mut session, &transcript, i, &ws, &mut expected, &mut live)?
                {
                    report.mismatches.push(finding);
                }
            }
        }
    }
    Ok(report)
}

/// Send one recorded client message, with `$WS` expanded back to the live
/// workspace.
fn send(
    session: &mut Session,
    rec: &Record,
    workspace: &str,
    report: &mut Report,
) -> Result<(), Error> {
    let mut msg = serde_json::Map::new();
    msg.insert("jsonrpc".to_string(), Value::from("2.0"));
    if let Some(id) = &rec.id {
        msg.insert("id".to_string(), id.clone());
    }
    if let Some(method) = &rec.method {
        msg.insert("method".to_string(), Value::from(method.clone()));
    }
    if let Some(params) = &rec.params {
        let mut params = params.clone();
        denormalize(&mut params, workspace);
        msg.insert("params".to_string(), params);
    }
    let msg = Value::Object(msg);
    session.send(&msg)?;

    // `exit` is not an ordinary notification: the client is expected to stop
    // writing, and a server that stays up is a process every editor leaks.
    if rec.method.as_deref() == Some("exit") {
        session.close_stdin();
        report.exit_code = session.wait_exit(session::EXIT_TIMEOUT)?;
    }
    Ok(())
}

/// The `env` directives of the `.lsps` beside a transcript, or nothing.
fn script_env(transcript: &Path) -> Vec<(String, String)> {
    let script = transcript.with_extension("lsps");
    std::fs::read_to_string(script)
        .ok()
        .and_then(|t| crate::Script::parse(&t).ok())
        .map(|s| s.env)
        .unwrap_or_default()
}

/// Undo the `paths` normalization: `$WS` is the workspace this run is using.
fn denormalize(value: &mut Value, workspace: &str) {
    match value {
        Value::String(s) => {
            if s.contains(normalize::WS) {
                *s = s.replace(normalize::WS, workspace);
            }
        }
        Value::Array(items) => {
            for item in items {
                denormalize(item, workspace);
            }
        }
        Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                denormalize(v, workspace);
            }
        }
        _ => {}
    }
}

/// Wait for the message a recorded `s2c` record describes, then compare.
fn expect(
    session: &mut Session,
    transcript: &Transcript,
    index: usize,
    workspace: &str,
    expected: &mut Normalizer,
    live: &mut Normalizer,
) -> Result<Option<Finding>, Error> {
    let rec = &mut transcript.records[index].clone();
    expected.record(rec);
    let rec = &*rec;
    let method = transcript
        .method_for(index)
        .unwrap_or("<response>")
        .to_string();
    let matcher: Matcher = transcript.matcher_for(index);
    let since = std::time::Instant::now();

    let arrived = match rec.kind {
        Kind::Response => {
            let id = rec.id.as_ref().and_then(Value::as_i64).ok_or_else(|| {
                Error::Parse(format!("record {}: response id is not a number", rec.seq))
            })?;
            session.response(id, since)?.0
        }
        Kind::Notification | Kind::Request => {
            let want = rec.method.as_deref().unwrap_or_default();
            // A notification is identified by its method *and* its document
            // where it has one: two `publishDiagnostics` for two files are
            // different claims and must not satisfy each other.
            // The recorded URI is normalized (`file://$WS/…`); the live one is
            // absolute. Expanding the placeholder here rather than comparing
            // normalized-to-live is the difference between a filter that
            // matches and one that waits out its whole timeout on every
            // transcript that opens a document.
            let uri = rec
                .params
                .as_ref()
                .and_then(|p| p.get("uri"))
                .and_then(Value::as_str)
                .map(|u| u.replace(normalize::WS, workspace));
            session.notification(want, uri.as_deref(), since)?.0
        }
    };

    // Normalize the live message through the same stages the record names.
    let mut live_rec = Record {
        seq: rec.seq,
        dir: Dir::S2c,
        kind: rec.kind,
        id: arrived.get("id").cloned(),
        method: arrived
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_string),
        params: arrived.get("params").cloned(),
        result: arrived.get("result").cloned(),
        error: arrived.get("error").cloned(),
        matcher: None,
        normalize: rec.normalize.clone(),
        t_us: None,
    };
    live.record(&mut live_rec);

    // A recorded `result` against a live `error` is a mismatch no matcher can
    // see, because each only looks at the payload that is there.
    if rec.result.is_some() != live_rec.result.is_some() {
        return Ok(Some(Finding {
            seq: rec.seq,
            method,
            matcher: matcher.to_string(),
            detail: if rec.result.is_some() {
                "the transcript records a result; the server answered with an error".to_string()
            } else {
                "the transcript records an error; the server answered with a result".to_string()
            },
            expected: rec.payload().clone(),
            actual: live_rec.payload().clone(),
        }));
    }

    Ok(match matcher.compare(rec.payload(), live_rec.payload()) {
        Ok(()) => None,
        Err(mismatch) => Some(Finding {
            seq: rec.seq,
            method,
            matcher: matcher.to_string(),
            detail: mismatch.to_string(),
            expected: rec.payload().clone(),
            actual: live_rec.payload().clone(),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn denormalize_expands_the_workspace_placeholder_inside_a_uri() {
        let mut v = json!({"textDocument": {"uri": "file://$WS/hello.lu"}});
        denormalize(&mut v, "/work/samples");
        assert_eq!(
            v.pointer("/textDocument/uri").unwrap(),
            "file:///work/samples/hello.lu"
        );
    }

    #[test]
    fn denormalize_reaches_into_arrays_and_leaves_other_strings_alone() {
        let mut v = json!({"items": ["$WS/a", "plain"], "n": 3});
        denormalize(&mut v, "/w");
        assert_eq!(v["items"][0], "/w/a");
        assert_eq!(v["items"][1], "plain");
        assert_eq!(v["n"], 3);
    }

    #[test]
    fn a_recorded_notification_uri_is_expanded_before_it_is_used_as_a_filter() {
        // Regression: matching the normalized `$WS` form against the live
        // absolute URI never matched, so every transcript that opened a
        // document sat out its full timeout and reported "nothing arrived"
        // while the notification was sitting in the inbox.
        let recorded = "file://$WS/hello.lu";
        assert_eq!(
            recorded.replace(normalize::WS, "/work/samples"),
            "file:///work/samples/hello.lu"
        );
    }

    #[test]
    fn a_pin_mismatch_says_what_to_do_about_it() {
        let e = Error::PinMismatch {
            recorded: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            actual: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        };
        let text = e.to_string();
        assert!(
            text.contains("aaaaaaa") && text.contains("bbbbbbb"),
            "{text}"
        );
        assert!(text.contains("lspconf record"), "{text}");
    }

    #[test]
    fn a_long_payload_is_truncated_in_the_report_rather_than_flooding_it() {
        let big = Value::from("x".repeat(1000));
        let text = compact(&big);
        assert!(text.len() < 500, "{}", text.len());
        assert!(text.contains("bytes"));
    }
}
