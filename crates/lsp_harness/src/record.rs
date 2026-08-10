//! `lspconf record` — drive a scripted session against the live server and
//! write the normalized transcript.
//!
//! # Why the committed transcripts carry no timings
//!
//! The record format has a `t_us` sidecar and this tool writes it — but not by
//! default, and the committed library is recorded without it. The reason is
//! the same one that motivates normalization at all: a transcript whose every
//! line changes on every re-record is a transcript nobody reviews, and the
//! whole design rests on a re-record producing a diff a human reads. Timings
//! are wall-clock noise by construction; putting them in the artifact would
//! bury the one line that actually changed under forty that did not.
//!
//! `--timings` writes them for a one-off capture. `lspconf bench` does not use
//! this path at all: it drives the same scripts through the same interpreter
//! and emits D5 JSONL, which is where a number that varies per run belongs.

use std::path::Path;

use lsp_transcript::record::{Dir, Header, Kind, Record, Transcript};
use lsp_transcript::{Normalizer, Stage, jsonl};
use serde_json::Value;

use crate::drive::{Driver, Event};
use crate::profiles::Profile;
use crate::script::Script;
use crate::session::Session;

/// Anything that stops a recording.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Script(String),
    Drive(crate::drive::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Script(m) => write!(f, "{m}"),
            Error::Drive(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<crate::drive::Error> for Error {
    fn from(e: crate::drive::Error) -> Self {
        Error::Drive(e)
    }
}

impl From<crate::session::Error> for Error {
    fn from(e: crate::session::Error) -> Self {
        Error::Drive(crate::drive::Error::Session(e))
    }
}

/// Knobs a recording takes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Write the `t_us` sidecar. Off for committed artifacts — see the module
    /// note.
    pub timings: bool,
}

/// Record one script into a transcript.
///
/// `recorded` is the ISO date stamped into the header; it is a parameter so a
/// re-record on a different day is a one-line diff rather than a surprise, and
/// so tests can pin it.
pub fn record(
    repo_root: &Path,
    bin: &Path,
    script_path: &Path,
    pin_commit: &str,
    recorded: &str,
    options: Options,
) -> Result<Transcript, Error> {
    let text = std::fs::read_to_string(script_path)?;
    let script = Script::parse(&text)
        .map_err(|e| Error::Script(format!("{}: {e}", crate::slash_path(script_path))))?;
    let profile_path = repo_root
        .join("profiles")
        .join(format!("{}.json", script.profile));
    let profile = Profile::load(&profile_path).map_err(|e| Error::Script(e.to_string()))?;

    let workspace = repo_root.join(&script.workspace);
    if !workspace.is_dir() {
        return Err(Error::Script(format!(
            "{}: workspace `{}` does not exist",
            crate::slash_path(script_path),
            script.workspace
        )));
    }
    let mut session = Session::spawn_with_env(bin, &workspace, &script.env)?;

    let mut records: Vec<Record> = Vec::new();
    let mut seq = 0u32;
    {
        let mut driver = Driver::new(&mut session, repo_root);
        let mut push = |event: Event| {
            seq += 1;
            records.push(to_record(seq, &event, options));
        };
        driver.run(&script, &profile.capabilities, &mut push)?;
    }

    let mut transcript = Transcript {
        header: Header {
            transcript: lsp_transcript::FORMAT_VERSION,
            name: script.name.clone(),
            wolf_pin: pin_commit.to_string(),
            profile: script.profile.clone(),
            workspace: script.workspace.clone(),
            recorded: recorded.to_string(),
        },
        records,
    };
    Normalizer::new(Some(workspace)).run(&mut transcript);
    Ok(transcript)
}

/// Write a transcript in canonical form, creating parent directories.
pub fn write(transcript: &Transcript, out: &Path) -> Result<(), Error> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, jsonl::to_string(transcript))?;
    Ok(())
}

/// One wire message → one record, with the normalization stages this repo's
/// policy attaches by method.
///
/// **Attached at record time, not at compare time, and visible in the file.**
/// A stage applied invisibly inside the matcher would be a rule a reviewer
/// reading the transcript cannot see, and the eliding rules are exactly the
/// thing most likely to be wrong.
fn to_record(seq: u32, event: &Event, options: Options) -> Record {
    let m = &event.message;
    let id = m.get("id").cloned();
    let method = m.get("method").and_then(Value::as_str).map(str::to_string);
    let result = m.get("result").cloned();
    let error = m.get("error").cloned();
    let params = m.get("params").cloned();

    let kind = if method.is_some() {
        if id.is_some() {
            Kind::Request
        } else {
            Kind::Notification
        }
    } else {
        Kind::Response
    };

    let mut normalize = Vec::new();
    if event.dir == Dir::S2c {
        match method.as_deref() {
            // The server's own version string changes on every release and
            // pins nothing about behavior.
            None => {
                if result
                    .as_ref()
                    .is_some_and(|r| r.get("serverInfo").is_some())
                {
                    normalize.push(Stage::ServerInfo);
                }
            }
            Some("textDocument/publishDiagnostics") => normalize.push(Stage::DiagSort),
            _ => {}
        }
    }

    Record {
        seq,
        dir: event.dir,
        kind,
        id,
        method,
        params,
        result,
        error,
        // Left absent on purpose: the default for the method is the contract
        // (`lsp_transcript::defaults`), and a recorder that stamped the
        // resolved matcher into every line would fossilize today's table into
        // forty files and make changing it a forty-file diff.
        matcher: None,
        normalize,
        t_us: if options.timings { event.t_us } else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(dir: Dir, message: Value) -> Event {
        Event {
            dir,
            message,
            t_us: Some(1234),
            step: 1,
        }
    }

    #[test]
    fn an_initialize_response_gets_the_serverinfo_stage() {
        let e = event(
            Dir::S2c,
            json!({"id": 1, "result": {"capabilities": {}, "serverInfo": {"version": "0.0.1"}}}),
        );
        let r = to_record(1, &e, Options::default());
        assert_eq!(r.kind, Kind::Response);
        assert_eq!(r.normalize, vec![Stage::ServerInfo]);
    }

    #[test]
    fn a_response_without_serverinfo_gets_no_stage() {
        let e = event(Dir::S2c, json!({"id": 2, "result": []}));
        assert!(to_record(1, &e, Options::default()).normalize.is_empty());
    }

    #[test]
    fn diagnostics_are_sorted_so_worker_order_does_not_churn_the_file() {
        let e = event(
            Dir::S2c,
            json!({"method": "textDocument/publishDiagnostics",
                   "params": {"uri": "file:///w/a.lu", "diagnostics": []}}),
        );
        let r = to_record(1, &e, Options::default());
        assert_eq!(r.kind, Kind::Notification);
        assert_eq!(r.normalize, vec![Stage::DiagSort]);
    }

    #[test]
    fn timings_are_omitted_unless_asked_for() {
        let e = event(Dir::S2c, json!({"id": 1, "result": null}));
        assert_eq!(to_record(1, &e, Options::default()).t_us, None);
        assert_eq!(to_record(1, &e, Options { timings: true }).t_us, Some(1234));
    }

    #[test]
    fn a_null_result_survives_as_a_present_null() {
        // `shutdown` really does answer `"result": null`, and a codec that
        // folded it to "no result" would make the record invalid.
        let e = event(Dir::S2c, json!({"id": 9, "result": null}));
        let r = to_record(1, &e, Options::default());
        assert_eq!(r.result, Some(Value::Null));
        assert!(r.error.is_none());
    }

    #[test]
    fn a_client_message_never_carries_normalization_of_its_own() {
        let e = event(
            Dir::C2s,
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {}}}),
        );
        assert!(to_record(1, &e, Options::default()).normalize.is_empty());
    }
}
