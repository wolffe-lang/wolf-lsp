//! `lspconf capture` — record a **real editor's** session by sitting between
//! it and the server.
//!
//! Every other transcript in this repo is *driven*: a `.lsps` script says what
//! to send and the harness sends it. That is the right shape for protocol
//! cases, and the wrong shape for the question ls02 onward has to answer —
//! *what does this editor actually send?* A hand-written script of what a
//! client is believed to do is a transcript of the belief.
//!
//! So this is a proxy, not a driver. It is `exec`'d **as** the server: the
//! editor spawns `wolf`, finds this on `PATH` first, and every byte it writes
//! is forwarded verbatim to the real binary and every byte back is forwarded
//! verbatim to the editor. Neither side is aware, and no instrumentation build
//! of the editor is needed — which matters because two of the four tier-0/1
//! clients cannot be rebuilt by CI at all.
//!
//! Three rules the forwarding obeys, all of them learned from the failure they
//! prevent:
//!
//! - **Forward bytes, then record.** The proxy must never be the reason a
//!   session behaves differently; a message it cannot parse is still
//!   forwarded, and only the recording of it is skipped.
//! - **Never write to stdout.** stdout *is* the protocol channel here. The
//!   proxy's own diagnostics go to stderr, and a stray `println!` would corrupt
//!   the very stream it exists to observe.
//! - **Do not drain into memory forever.** The transcript is the point, so
//!   messages are kept — but a capture is an interactive session a human ends,
//!   not a service.
//!
//! The result is a transcript with **no `.lsps` beside it**, which is exactly
//! what makes it evidence rather than a script's echo. `lspconf verify` knows
//! about that shape (`transcripts/<client>/`), and `replay` already tolerates
//! it.

use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use lsp_transcript::record::{Dir, Header, Kind, Record, Transcript};
use lsp_transcript::{Normalizer, Stage};
use serde_json::Value;

use crate::framing::{FrameReader, write_frame};

/// Anything that stops a capture.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Spawn {
        program: String,
        source: std::io::Error,
    },
    Config(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Spawn { program, source } => write!(f, "cannot spawn `{program}`: {source}"),
            Error::Config(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// What a capture needs to know that the wire cannot tell it.
#[derive(Debug, Clone)]
pub struct Options {
    /// `<client>/<scenario>`, the transcript's name. Its first segment is what
    /// marks the file client-recorded.
    pub name: String,
    /// The capability profile this client was read for.
    pub profile: String,
    /// Repo-relative workspace, recorded in the header and used to elide paths.
    pub workspace: String,
    /// The pin the session ran against.
    pub pin_commit: String,
    /// ISO date stamped into the header.
    pub recorded: String,
    /// Absolute path of `workspace`, for eliding it to `$WS`.
    ///
    /// Without this, every URI the editor sent carries the recording machine's
    /// home directory and the transcript replays on exactly one computer.
    pub workspace_dir: PathBuf,
}

/// One observed message, in arrival order across both directions.
struct Observed {
    /// The order this frame was **read off the wire**, across both pumps.
    ///
    /// Not the order it reached the sink, and the difference is a bug this
    /// module shipped with. The pumps forward first and record second (the
    /// proxy must never be the reason a session behaves differently), so
    /// between a request being forwarded and being recorded, the server can
    /// answer it and the downward pump can record the *response* first. The
    /// result is a transcript whose `initialize` response precedes its
    /// `initialize` request — nondeterministically, so it survives one
    /// re-record and reappears on the next.
    ///
    /// A ticket taken at read time fixes it without moving the write: the
    /// number is claimed before the frame is forwarded, so causality is
    /// exact (a response cannot be read before the request that caused it was
    /// read), and the cost is one relaxed atomic increment rather than a disk
    /// write in the forwarding path.
    order: u64,
    dir: Dir,
    message: Value,
}

/// Proxy `stdin`/`stdout` to `command`, recording both directions into `out`.
///
/// **The file is rewritten after every message, not once at the end.** An
/// editor's idea of stopping a language server is `shutdown`, `exit`, and then
/// SIGKILL of the process it spawned — which is this proxy — and the kill wins
/// that race every time. A capture that only wrote on a clean exit would
/// record nothing at all from the clients most worth recording. The rewrite is
/// O(n²) in messages across a session; a capture is a deliberate recording of
/// a bounded one, and an empty file is the worse trade.
pub fn capture(command: &[String], options: &Options, out: &Path) -> Result<Transcript, Error> {
    let Some((program, args)) = command.split_first() else {
        return Err(Error::Config(
            "capture needs a command to proxy, after `--`".to_string(),
        ));
    };

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // stderr is inherited: the server's own stderr belongs to whoever ran
        // the editor, and swallowing it here would hide a server that died.
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|source| Error::Spawn {
            program: program.clone(),
            source,
        })?;

    let child_stdin = child.stdin.take().expect("piped");
    let child_stdout = child.stdout.take().expect("piped");

    let observed: Arc<Mutex<Vec<Observed>>> = Arc::new(Mutex::new(Vec::new()));

    // The editor→server direction is **detached on purpose.** Editors do not
    // politely close their end: fackr sends `shutdown`/`exit` and then SIGKILLs
    // the process it spawned — which is this proxy. Joining the upward pump
    // would park here until that kill arrived, and a killed process writes no
    // transcript. The downward pump ending means the server is gone, which is
    // the real end of the session; anything the editor writes after that has
    // nowhere to go anyway.
    let sink = Sink {
        observed: Arc::clone(&observed),
        // Shared across both pumps: the ticket only orders the two directions
        // against each other, so a per-pump counter would order nothing.
        order: Arc::new(AtomicU64::new(0)),
        options: options.clone(),
        out: out.to_path_buf(),
    };
    std::thread::spawn({
        let sink = sink.clone();
        move || pump(std::io::stdin(), child_stdin, Dir::C2s, &sink)
    });
    let down = std::thread::spawn({
        let sink = sink.clone();
        move || pump(child_stdout, std::io::stdout(), Dir::S2c, &sink)
    });

    let _ = down.join();
    // The server going away does not mean the editor has finished writing:
    // `exit` is sent, then the server dies, and the notification is still in
    // flight up here. A short grace lets the last frames land rather than
    // truncating the transcript one message before its end.
    std::thread::sleep(std::time::Duration::from_millis(100));
    reap(&mut child);

    let transcript = sink.transcript();
    write(&transcript, out)?;
    Ok(transcript)
}

/// Where observed messages accumulate, and how they reach disk.
#[derive(Clone)]
struct Sink {
    observed: Arc<Mutex<Vec<Observed>>>,
    order: Arc<AtomicU64>,
    options: Options,
    out: PathBuf,
}

impl Sink {
    /// Claim the next wire-order ticket. Called at frame-read time, before
    /// the frame is forwarded.
    fn ticket(&self) -> u64 {
        self.order.fetch_add(1, Ordering::Relaxed)
    }

    /// Record one message and persist the transcript so far.
    fn push(&self, message: Observed) {
        match self.observed.lock() {
            Ok(mut observed) => observed.push(message),
            Err(_) => return,
        }
        let transcript = self.transcript();
        if let Err(e) = write(&transcript, &self.out) {
            eprintln!("lspconf capture: cannot write the transcript: {e}");
        }
    }

    fn transcript(&self) -> Transcript {
        let observed = self.observed.lock().expect("observed mutex");
        // Wire order, not sink order. The two differ whenever a response
        // overtakes the recording of its own request.
        let mut ordered: Vec<&Observed> = observed.iter().collect();
        ordered.sort_by_key(|o| o.order);
        let mut transcript = Transcript {
            header: Header {
                transcript: lsp_transcript::FORMAT_VERSION,
                name: self.options.name.clone(),
                wolf_pin: self.options.pin_commit.clone(),
                profile: self.options.profile.clone(),
                workspace: self.options.workspace.clone(),
                recorded: self.options.recorded.clone(),
            },
            records: ordered
                .iter()
                .enumerate()
                .map(|(i, o)| to_record(i as u32 + 1, o))
                .collect(),
        };
        Normalizer::new(Some(self.options.workspace_dir.clone())).run(&mut transcript);
        transcript
    }
}

/// A capture ends when the editor hangs up; give the child a moment to exit on
/// its own `exit` notification before insisting.
fn reap(child: &mut Child) {
    for _ in 0..50 {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Forward every frame from `from` to `to`, reporting each one.
///
/// A frame that is not JSON is still forwarded — the proxy's job is to be
/// invisible, and a message it cannot model is a message the transcript simply
/// does not carry (with a note on stderr, where a human will see it).
fn pump<R: Read, W: Write>(from: R, mut to: W, dir: Dir, sink: &Sink) {
    let mut reader = FrameReader::new(BufReader::new(from));
    loop {
        match reader.read_frame() {
            Ok(Some(body)) => {
                // Claimed BEFORE the forward, so the number reflects when the
                // frame was read rather than when this thread got around to
                // recording it.
                let order = sink.ticket();
                if write_frame(&mut to, &body).is_err() {
                    return;
                }
                match serde_json::from_slice::<Value>(&body) {
                    Ok(message) => sink.push(Observed {
                        order,
                        dir,
                        message,
                    }),
                    Err(e) => eprintln!("lspconf capture: {dir:?} frame is not JSON: {e}"),
                }
            }
            Ok(None) => return,
            Err(e) => {
                eprintln!("lspconf capture: {dir:?} framing error: {e}");
                return;
            }
        }
    }
}

/// One observed message → one record.
///
/// Mirrors `record::to_record`'s policy so a captured transcript and a driven
/// one carry the same normalization for the same method; the difference
/// between the two files should be what the *client* did, never how it was
/// written down.
fn to_record(seq: u32, o: &Observed) -> Record {
    let m = &o.message;
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
    if o.dir == Dir::S2c {
        match method.as_deref() {
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
    // Deliberately no `Stage::Uri`. A capture's `c2s` records are *sent* on
    // replay, and replay only rehydrates `$WS` (the unconditional `paths`
    // stage) — a record elided to `$URI` would be replayed as the literal
    // string. Path elision already makes the file machine-independent, and it
    // is the only elision that survives a round trip.
    Record {
        seq,
        dir: o.dir,
        kind,
        id,
        method,
        params,
        result,
        error,
        matcher: None,
        normalize,
        t_us: None,
    }
}

/// Write a captured transcript in canonical form.
pub fn write(transcript: &Transcript, out: &Path) -> Result<(), Error> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, lsp_transcript::jsonl::to_string(transcript))?;
    Ok(())
}

/// Where a capture writes when `--out` is not given.
pub fn default_out(repo_root: &Path, name: &str) -> PathBuf {
    repo_root.join("transcripts").join(format!("{name}.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ordering bug this module shipped with, pinned.
    ///
    /// The pumps forward before they record, so a response can reach the sink
    /// ahead of the request that caused it — producing a transcript whose
    /// `initialize` response is `seq: 1`. It is a race, so it reproduces
    /// intermittently and survives a re-record, which is the worst shape a bug
    /// in a recording tool can have. `transcript()` sorts by the ticket each
    /// frame took at READ time, and this test feeds the sink in the wrong order
    /// on purpose.
    #[test]
    fn a_response_recorded_before_its_request_is_still_ordered_after_it() {
        let sink = Sink {
            observed: Arc::new(Mutex::new(Vec::new())),
            order: Arc::new(AtomicU64::new(0)),
            options: Options {
                name: "client/scenario".to_string(),
                profile: "minimal".to_string(),
                workspace: "vendor/upstream/samples".to_string(),
                pin_commit: "0".repeat(40),
                recorded: "2026-08-10".to_string(),
                workspace_dir: PathBuf::from("/ws"),
            },
            out: PathBuf::from("/dev/null"),
        };

        // Tickets in wire order: the request was read first.
        let request_ticket = sink.ticket();
        let response_ticket = sink.ticket();
        assert!(request_ticket < response_ticket);

        // Sink order is the reverse — the race.
        sink.observed.lock().expect("mutex").push(Observed {
            order: response_ticket,
            dir: Dir::S2c,
            message: serde_json::json!({"id": 1, "result": {}}),
        });
        sink.observed.lock().expect("mutex").push(Observed {
            order: request_ticket,
            dir: Dir::C2s,
            message: serde_json::json!({"id": 1, "method": "initialize", "params": {}}),
        });

        let transcript = sink.transcript();
        assert_eq!(transcript.records[0].method.as_deref(), Some("initialize"));
        assert_eq!(transcript.records[0].dir, Dir::C2s);
        assert_eq!(transcript.records[0].seq, 1);
        assert_eq!(transcript.records[1].dir, Dir::S2c);
        assert_eq!(transcript.records[1].seq, 2);
    }

    #[test]
    fn requests_notifications_and_responses_are_told_apart() {
        let request = to_record(
            1,
            &Observed {
                order: 0,
                dir: Dir::C2s,
                message: serde_json::json!({"id": 1, "method": "initialize", "params": {}}),
            },
        );
        assert_eq!(request.kind, Kind::Request);

        let notification = to_record(
            2,
            &Observed {
                order: 0,
                dir: Dir::C2s,
                message: serde_json::json!({"method": "initialized", "params": {}}),
            },
        );
        assert_eq!(notification.kind, Kind::Notification);
        assert!(notification.id.is_none());

        let response = to_record(
            3,
            &Observed {
                order: 0,
                dir: Dir::S2c,
                message: serde_json::json!({"id": 1, "result": {"serverInfo": {"name": "x"}}}),
            },
        );
        assert_eq!(response.kind, Kind::Response);
        assert!(response.normalize.contains(&Stage::ServerInfo));
    }

    /// A capture must **not** ask for `uri` elision, however tempting it looks
    /// on a file full of one machine's home directory. `c2s` records are
    /// *replayed*, and replay only rehydrates `$WS`; a `$URI` placeholder would
    /// be sent to the server as that literal string. Path elision (applied
    /// unconditionally, with the workspace this ran in) is the elision that
    /// survives the round trip.
    #[test]
    fn uris_are_left_for_the_path_stage() {
        let r = to_record(
            1,
            &Observed {
                order: 0,
                dir: Dir::C2s,
                message: serde_json::json!({
                    "method": "textDocument/didOpen",
                    "params": {"textDocument": {"uri": "file:///home/x/a.lu"}}
                }),
            },
        );
        assert!(!r.normalize.contains(&Stage::Uri));
    }

    #[test]
    fn diagnostics_are_sorted_like_a_driven_recording() {
        let r = to_record(
            1,
            &Observed {
                order: 0,
                dir: Dir::S2c,
                message: serde_json::json!({
                    "method": "textDocument/publishDiagnostics",
                    "params": {"uri": "file:///a.lu", "diagnostics": []}
                }),
            },
        );
        assert!(r.normalize.contains(&Stage::DiagSort));
    }
}
