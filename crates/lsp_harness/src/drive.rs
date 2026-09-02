//! Executing a [`Script`] against a live [`Session`], and telling someone
//! about every message that crossed the wire.
//!
//! One interpreter, three consumers. `lspconf record` turns the event stream
//! into a transcript, `lspconf bench` turns it into latency samples, and the
//! §5/§6 tests use it to set up a state and then assert against it directly.
//! Writing three drivers would guarantee three different notions of "what a
//! `didChange` is", which is precisely the bug class the harness is for.
//!
//! The interpreter owns two pieces of state the DSL leans on:
//!
//! - **Document text**, so `splice` can describe an edit as a byte range over
//!   what the buffer currently holds rather than as a whole new file.
//! - **Document version**, monotonic per document, so version discipline is
//!   the default and violating it takes an explicit `edit-version`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lsp_transcript::record::Dir;
use serde_json::{Value, json};

use crate::script::{Req, Script, Step};
use crate::session::{self, Session, file_uri};

/// One message crossing the wire, with the timing sidecar.
#[derive(Debug, Clone)]
pub struct Event {
    pub dir: Dir,
    pub message: Value,
    /// Microseconds from the causally preceding client message: for a
    /// response, from the request that shares its id; for a notification,
    /// from the last thing the client sent. `None` on `c2s`, which measures
    /// nothing.
    pub t_us: Option<u64>,
    /// The step that produced it, for the bench's request-class bucketing.
    pub step: usize,
}

/// What the run produced besides its events.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// Process exit code, when the script ran the process to completion.
    pub exit_code: Option<i32>,
    /// The `initialize` result, verbatim.
    pub initialize_result: Option<Value>,
    /// Final text of every document the script touched, keyed by script-
    /// relative name — the round-trip oracle's baseline.
    pub documents: BTreeMap<String, String>,
}

/// Errors the interpreter itself can raise, on top of the session's.
#[derive(Debug)]
pub enum Error {
    Session(session::Error),
    /// A step referred to something the script never established.
    Script {
        step: usize,
        message: String,
    },
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Session(e) => write!(f, "{e}"),
            Error::Script { step, message } => write!(f, "step {step}: {message}"),
            Error::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<session::Error> for Error {
    fn from(e: session::Error) -> Self {
        Error::Session(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// A document the session has open.
struct Doc {
    uri: String,
    text: Vec<u8>,
    version: i64,
}

/// The script interpreter.
pub struct Driver<'a> {
    session: &'a mut Session,
    repo_root: PathBuf,
    workspace: PathBuf,
    docs: BTreeMap<String, Doc>,
    /// When each outstanding request was sent, for per-response latency.
    sent_at: BTreeMap<i64, Instant>,
    last_send: Instant,
    cursor: usize,
    step: usize,
}

impl<'a> Driver<'a> {
    /// Build an interpreter for `session`, whose workspace `script` names.
    pub fn new(session: &'a mut Session, repo_root: &Path) -> Self {
        let workspace = session.workspace().to_path_buf();
        let cursor = session.log_len();
        Self {
            session,
            repo_root: repo_root.to_path_buf(),
            workspace,
            docs: BTreeMap::new(),
            sent_at: BTreeMap::new(),
            last_send: Instant::now(),
            cursor,
            step: 0,
        }
    }

    /// Run a whole script, reporting every message to `on_event`.
    pub fn run(
        &mut self,
        script: &Script,
        capabilities: &Value,
        on_event: &mut dyn FnMut(Event),
    ) -> Result<Outcome, Error> {
        let mut outcome = Outcome::default();
        for (i, step) in script.steps.iter().enumerate() {
            self.step = i + 1;
            self.step(step, capabilities, &mut outcome, on_event)?;
        }
        for (name, doc) in &self.docs {
            outcome.documents.insert(
                name.clone(),
                String::from_utf8_lossy(&doc.text).into_owned(),
            );
        }
        Ok(outcome)
    }

    /// The current text of an open document, for the oracles.
    #[must_use]
    pub fn document(&self, file: &str) -> Option<&[u8]> {
        self.docs.get(file).map(|d| d.text.as_slice())
    }

    /// The URI the session used for a document.
    #[must_use]
    pub fn uri_of(&self, file: &str) -> Option<&str> {
        self.docs.get(file).map(|d| d.uri.as_str())
    }

    fn step(
        &mut self,
        step: &Step,
        capabilities: &Value,
        outcome: &mut Outcome,
        on_event: &mut dyn FnMut(Event),
    ) -> Result<(), Error> {
        match step {
            Step::Initialize => {
                let started = Instant::now();
                self.last_send = started;
                // `Session::initialize` sends two messages and consumes one;
                // reconstructing them for the transcript rather than
                // duplicating the handshake keeps one implementation of it.
                let result = self.session.initialize(capabilities)?;
                let id = result.get("id").and_then(Value::as_i64).unwrap_or(1);
                self.sent_at.insert(id, started);
                self.emit_c2s(
                    json!({"jsonrpc": "2.0", "id": id, "method": "initialize",
                           "params": self.initialize_params(capabilities)}),
                    on_event,
                );
                self.flush(on_event);
                self.emit_c2s(
                    json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
                    on_event,
                );
                outcome.initialize_result = Some(result);
            }
            Step::Open { file, source } => {
                let text = self.read_source(source)?;
                let uri = file_uri(&self.workspace.join(file));
                self.docs.insert(
                    file.clone(),
                    Doc {
                        uri: uri.clone(),
                        text: text.clone(),
                        version: 1,
                    },
                );
                self.send_notification(
                    "textDocument/didOpen",
                    json!({"textDocument": {
                        "uri": uri, "languageId": "wolf", "version": 1,
                        "text": String::from_utf8_lossy(&text)}}),
                    on_event,
                )?;
            }
            Step::Edit {
                file,
                text,
                version,
            } => {
                self.change(file, text.clone().into_bytes(), *version, on_event)?;
            }
            Step::EditFrom { file, source } => {
                let text = self.read_source(source)?;
                self.change(file, text, None, on_event)?;
            }
            Step::Splice { file, lo, hi, text } => {
                let current = self
                    .docs
                    .get(file)
                    .ok_or_else(|| self.unopened(file))?
                    .text
                    .clone();
                let spliced = splice(&current, *lo, *hi, text.as_bytes());
                self.change(file, spliced, None, on_event)?;
            }
            Step::Save { file, include_text } => {
                let doc = self.docs.get(file).ok_or_else(|| self.unopened(file))?;
                let mut params = json!({"textDocument": {"uri": doc.uri}});
                if *include_text {
                    params["text"] = Value::from(String::from_utf8_lossy(&doc.text).into_owned());
                }
                self.send_notification("textDocument/didSave", params, on_event)?;
            }
            Step::Close { file } => {
                let doc = self.docs.remove(file).ok_or_else(|| self.unopened(file))?;
                self.send_notification(
                    "textDocument/didClose",
                    json!({"textDocument": {"uri": doc.uri}}),
                    on_event,
                )?;
            }
            Step::Request(req) => {
                let id = self.send_request(req, on_event)?;
                let since = self.sent_at[&id];
                let _ = self.session.response(id, since)?;
                self.flush(on_event);
            }
            Step::Send(req) => {
                self.send_request(req, on_event)?;
            }
            Step::Cancel { id } => {
                self.send_notification("$/cancelRequest", json!({"id": id}), on_event)?;
            }
            Step::WaitDiagnostics { file } => {
                let uri = self
                    .docs
                    .get(file)
                    .map(|d| d.uri.clone())
                    .ok_or_else(|| self.unopened(file))?;
                let since = self.last_send;
                let _ = self.session.notification(
                    "textDocument/publishDiagnostics",
                    Some(&uri),
                    since,
                )?;
                self.flush(on_event);
            }
            Step::WaitResponse { id } => {
                let since = *self.sent_at.get(id).unwrap_or(&self.last_send);
                let _ = self.session.response(*id, since)?;
                self.flush(on_event);
            }
            Step::WaitQuiet { ms } => {
                let _ = self.session.drain(Duration::from_millis(*ms))?;
                self.flush(on_event);
            }
            Step::Sleep { ms } => {
                std::thread::sleep(Duration::from_millis(*ms));
                self.flush(on_event);
            }
            Step::Shutdown => {
                let id = self.session.next_id();
                let started = Instant::now();
                self.last_send = started;
                self.sent_at.insert(id, started);
                let msg = json!({"jsonrpc": "2.0", "id": id, "method": "shutdown",
                                 "params": Value::Null});
                self.session.send(&msg)?;
                self.emit_c2s(msg, on_event);
                let _ = self.session.response(id, started)?;
                self.flush(on_event);
            }
            Step::Exit => {
                self.send_notification_value("exit", Value::Null, on_event)?;
                self.session.close_stdin();
                outcome.exit_code = self.session.wait_exit(session::EXIT_TIMEOUT)?;
                self.flush(on_event);
            }
            Step::ShutdownExit => {
                self.step(&Step::Shutdown, capabilities, outcome, on_event)?;
                self.step(&Step::Exit, capabilities, outcome, on_event)?;
            }
            Step::Raw { bytes } => {
                self.last_send = Instant::now();
                self.session.send_raw(bytes)?;
                self.flush(on_event);
            }
        }
        Ok(())
    }

    // ------------------------------------------------------- primitives --

    fn unopened(&self, file: &str) -> Error {
        Error::Script {
            step: self.step,
            message: format!(
                "`{file}` is not open — a script must `open` a document before editing, \
                 saving, closing, or waiting on it"
            ),
        }
    }

    /// Resolve a source path: workspace-relative first, then repo-relative.
    ///
    /// The order matters. Corpus samples are the normal case and must win;
    /// the repo-relative fallback exists for the local fixtures that fill a
    /// gap `samples.toml` records, and nothing else.
    fn read_source(&self, source: &str) -> Result<Vec<u8>, Error> {
        let in_ws = self.workspace.join(source);
        if in_ws.is_file() {
            return Ok(std::fs::read(in_ws)?);
        }
        let in_repo = self.repo_root.join(source);
        if in_repo.is_file() {
            return Ok(std::fs::read(in_repo)?);
        }
        Err(Error::Script {
            step: self.step,
            message: format!(
                "no such source `{source}`: looked in {} and {}",
                crate::slash_path(&self.workspace),
                crate::slash_path(&self.repo_root)
            ),
        })
    }

    fn initialize_params(&self, capabilities: &Value) -> Value {
        json!({
            "processId": std::process::id(),
            "rootUri": file_uri(&self.workspace),
            "workspaceFolders": [{"uri": file_uri(&self.workspace), "name": "samples"}],
            "capabilities": capabilities,
            "clientInfo": {"name": "lspconf", "version": env!("CARGO_PKG_VERSION")},
        })
    }

    fn change(
        &mut self,
        file: &str,
        text: Vec<u8>,
        version: Option<i64>,
        on_event: &mut dyn FnMut(Event),
    ) -> Result<(), Error> {
        let doc = self.docs.get_mut(file).ok_or_else(|| Error::Script {
            step: self.step,
            message: format!("`{file}` is not open"),
        })?;
        doc.text = text.clone();
        // Monotonic by default; `edit-version` is the only way to send a stale
        // one, and §6 uses it on purpose.
        doc.version = version.unwrap_or(doc.version + 1);
        let params = json!({
            "textDocument": {"uri": doc.uri, "version": doc.version},
            "contentChanges": [{"text": String::from_utf8_lossy(&text)}],
        });
        self.send_notification("textDocument/didChange", params, on_event)
    }

    fn send_request(&mut self, req: &Req, on_event: &mut dyn FnMut(Event)) -> Result<i64, Error> {
        let params = self.req_params(req)?;
        let id = self.session.next_id();
        let now = Instant::now();
        self.sent_at.insert(id, now);
        self.last_send = now;
        let msg = json!({"jsonrpc": "2.0", "id": id, "method": req.method(), "params": params});
        self.session.send(&msg)?;
        self.emit_c2s(msg, on_event);
        Ok(id)
    }

    fn req_params(&self, req: &Req) -> Result<Value, Error> {
        let uri = |file: &str| -> Result<String, Error> {
            self.docs
                .get(file)
                .map(|d| d.uri.clone())
                // A request against a document the session never opened is
                // legal LSP and something clients really do; the URI is then
                // just the workspace path.
                .or_else(|| Some(file_uri(&self.workspace.join(file))))
                .ok_or_else(|| self.unopened(file))
        };
        Ok(match req {
            Req::Hover {
                file,
                line,
                character,
            } => json!({"textDocument": {"uri": uri(file)?},
                        "position": {"line": line, "character": character}}),
            Req::DocumentSymbol { file } => json!({"textDocument": {"uri": uri(file)?}}),
            Req::Formatting { file } => json!({"textDocument": {"uri": uri(file)?},
                        "options": {"tabSize": 4, "insertSpaces": true}}),
            Req::CodeAction { file, start, end } => json!({
                "textDocument": {"uri": uri(file)?},
                "range": {"start": {"line": start.0, "character": start.1},
                          "end": {"line": end.0, "character": end.1}},
                "context": {"diagnostics": []},
            }),
            Req::Definition {
                file,
                line,
                character,
            }
            | Req::PrepareRename {
                file,
                line,
                character,
            }
            | Req::SignatureHelp {
                file,
                line,
                character,
            } => json!({"textDocument": {"uri": uri(file)?},
                        "position": {"line": line, "character": character}}),
            Req::SemanticTokens { file, range: None } => {
                json!({"textDocument": {"uri": uri(file)?}})
            }
            Req::SemanticTokens {
                file,
                range: Some((start, end)),
            }
            | Req::InlayHint { file, start, end } => json!({
                "textDocument": {"uri": uri(file)?},
                "range": {"start": {"line": start.0, "character": start.1},
                          "end": {"line": end.0, "character": end.1}},
            }),
            Req::References {
                file,
                line,
                character,
                include_declaration,
            } => json!({"textDocument": {"uri": uri(file)?},
                        "position": {"line": line, "character": character},
                        "context": {"includeDeclaration": include_declaration}}),
            Req::Rename {
                file,
                line,
                character,
                new_name,
            } => json!({"textDocument": {"uri": uri(file)?},
                        "position": {"line": line, "character": character},
                        "newName": new_name}),
            // `$WS` is expanded here so a script can write a raw request
            // against a real document without hard-coding an absolute path
            // that would differ on every machine. The same placeholder the
            // `paths` normalization produces, going the other way.
            Req::Raw { params, .. } => {
                let mut params = params.clone();
                expand_ws(&mut params, &crate::slash_path(&self.workspace));
                params
            }
        })
    }

    fn send_notification(
        &mut self,
        method: &str,
        params: Value,
        on_event: &mut dyn FnMut(Event),
    ) -> Result<(), Error> {
        self.send_notification_value(method, params, on_event)
    }

    fn send_notification_value(
        &mut self,
        method: &str,
        params: Value,
        on_event: &mut dyn FnMut(Event),
    ) -> Result<(), Error> {
        self.last_send = Instant::now();
        let msg = json!({"jsonrpc": "2.0", "method": method, "params": params});
        self.session.send(&msg)?;
        self.emit_c2s(msg, on_event);
        Ok(())
    }

    fn emit_c2s(&mut self, message: Value, on_event: &mut dyn FnMut(Event)) {
        on_event(Event {
            dir: Dir::C2s,
            message,
            t_us: None,
            step: self.step,
        });
    }

    /// Report every message that has arrived since the last flush, in arrival
    /// order, with its latency measured from the message that caused it.
    fn flush(&mut self, on_event: &mut dyn FnMut(Event)) {
        let from = self.cursor;
        let events: Vec<Event> = self
            .session
            .log_from(from)
            .iter()
            .map(|(message, at)| {
                let origin = message
                    .get("id")
                    .and_then(Value::as_i64)
                    .and_then(|id| self.sent_at.get(&id).copied())
                    .unwrap_or(self.last_send);
                Event {
                    dir: Dir::S2c,
                    message: message.clone(),
                    t_us: Some(
                        at.saturating_duration_since(origin)
                            .as_micros()
                            .try_into()
                            .unwrap_or(u64::MAX),
                    ),
                    step: self.step,
                }
            })
            .collect();
        self.cursor = self.session.log_len();
        for e in events {
            on_event(e);
        }
    }
}

/// Expand the `$WS` placeholder inside every string of a value.
fn expand_ws(value: &mut Value, workspace: &str) {
    match value {
        Value::String(s) => {
            if s.contains(lsp_transcript::normalize::WS) {
                *s = s.replace(lsp_transcript::normalize::WS, workspace);
            }
        }
        Value::Array(items) => {
            for item in items {
                expand_ws(item, workspace);
            }
        }
        Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                expand_ws(v, workspace);
            }
        }
        _ => {}
    }
}

/// Replace `[lo, hi)` of `src` with `with`, clamping both ends.
///
/// Clamping rather than asserting is deliberate: the fuzzer generates offsets
/// against a document it believes is current, and a harness that panics on a
/// stale offset would report a harness bug as a server bug.
#[must_use]
pub fn splice(src: &[u8], lo: usize, hi: usize, with: &[u8]) -> Vec<u8> {
    let lo = lo.min(src.len());
    let hi = hi.clamp(lo, src.len());
    let mut out = Vec::with_capacity(src.len() - (hi - lo) + with.len());
    out.extend_from_slice(&src[..lo]);
    out.extend_from_slice(with);
    out.extend_from_slice(&src[hi..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_params_get_the_workspace_placeholder_expanded() {
        let mut v = json!({"textDocument": {"uri": "file://$WS/hello.lu"}, "n": 1});
        expand_ws(&mut v, "/work/samples");
        assert_eq!(
            v.pointer("/textDocument/uri").unwrap(),
            "file:///work/samples/hello.lu"
        );
        assert_eq!(v["n"], 1);
    }

    #[test]
    fn splice_replaces_a_range() {
        assert_eq!(splice(b"abcdef", 2, 4, b"XY"), b"abXYef".to_vec());
        assert_eq!(splice(b"abcdef", 2, 2, b"X"), b"abXcdef".to_vec());
        assert_eq!(splice(b"abcdef", 0, 6, b""), b"".to_vec());
    }

    #[test]
    fn splice_clamps_rather_than_panicking() {
        // The fuzzer computes offsets against what it believes is current; a
        // panic here would report a harness bug as a server bug.
        assert_eq!(splice(b"abc", 99, 99, b"X"), b"abcX".to_vec());
        // `hi` below `lo` collapses to an insertion at `lo`, never a reversal.
        assert_eq!(splice(b"abc", 2, 0, b"X"), b"abXc".to_vec());
    }

    #[test]
    fn splice_is_invertible_which_is_what_the_round_trip_oracle_leans_on() {
        let src = b"fn main() -> !int { 0 }".to_vec();
        let inserted = splice(&src, 4, 4, b"XYZ");
        assert_eq!(splice(&inserted, 4, 7, b""), src);
    }
}
