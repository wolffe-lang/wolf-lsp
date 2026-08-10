//! A live `wolf lsp` session: spawn, frame, correlate, and never hang.
//!
//! This is the module ls00 deferred — everything above [`crate::framing`] that
//! needs a process on the other end. Three properties are load-bearing and
//! each of them is a bug this design refuses to have:
//!
//! **Responses arrive out of order.** The server dispatches every request to
//! its own worker thread, so a `documentSymbol` issued before a `hover` can
//! answer after it, and a `publishDiagnostics` can land in the middle of
//! either. A driver that reads one message and expects it to be *the* answer
//! is testing a race. Everything here correlates: responses by `id`,
//! notifications by method, and unclaimed messages stay in the [`Session`]'s
//! inbox for whoever wants them next.
//!
//! **A dead server must not present as a slow one.** The reader thread reports
//! end-of-stream as an explicit event and stderr is drained continuously into
//! a buffer, so a panicking server produces "the server exited, and here is
//! what it said" rather than a timeout with no evidence. Draining also matters
//! for its own sake: a server whose stderr nobody reads deadlocks at the pipe
//! buffer — report 09 records fackr doing exactly that at ~64 KiB.
//!
//! **Every wait is bounded.** There is no blocking receive anywhere in this
//! file. A harness that can hang is a CI job that burns its whole timeout and
//! reports nothing, which is strictly worse than a fast wrong answer.

use std::collections::VecDeque;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use lsp_transcript::Encoding;
use serde_json::{Value, json};

use crate::framing::{self, FrameReader};

/// Default per-message wait. Generous on purpose: the v0 server is the
/// *non-resident* compiler answering from a cold query host, and report 09's
/// cold budget is 5 s. A tight timeout here would fail the suite for being
/// slow, which is what §8 measures and does not gate.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

/// How long to wait for the process to leave after `exit`.
pub const EXIT_TIMEOUT: Duration = Duration::from_secs(10);

/// Something went wrong that is not a mismatch.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// Nothing matching arrived in time. Carries what was awaited and
    /// everything that *did* arrive, because "timeout" alone is unactionable.
    Timeout {
        awaited: String,
        seen: Vec<String>,
        stderr: String,
    },
    /// The server closed its stdout, i.e. it exited.
    ServerGone {
        awaited: String,
        stderr: String,
    },
    /// A frame arrived that is not JSON, or not a JSON-RPC message.
    Protocol(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Timeout {
                awaited,
                seen,
                stderr,
            } => {
                writeln!(f, "timed out waiting for {awaited}")?;
                if seen.is_empty() {
                    writeln!(f, "  nothing arrived at all")?;
                } else {
                    writeln!(f, "  what did arrive, in order:")?;
                    for s in seen {
                        writeln!(f, "    {s}")?;
                    }
                }
                write_stderr(f, stderr)
            }
            Error::ServerGone { awaited, stderr } => {
                writeln!(
                    f,
                    "the server closed its stdout while waiting for {awaited} \
                     — it exited rather than answering"
                )?;
                write_stderr(f, stderr)
            }
            Error::Protocol(m) => write!(f, "protocol: {m}"),
        }
    }
}

fn write_stderr(f: &mut std::fmt::Formatter<'_>, stderr: &str) -> std::fmt::Result {
    if stderr.trim().is_empty() {
        writeln!(f, "  server stderr: (empty)")
    } else {
        writeln!(f, "  server stderr:")?;
        for line in stderr.lines() {
            writeln!(f, "    {line}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// What the reader thread hands back.
enum Incoming {
    Message(Value),
    /// The stream ended cleanly, or the frame was unreadable. Either way no
    /// further message will arrive.
    End(Option<String>),
}

/// One live server process.
pub struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<Incoming>,
    reader: Option<JoinHandle<()>>,
    stderr_pump: Option<JoinHandle<()>>,
    stderr: Arc<Mutex<String>>,
    /// Messages that arrived but no waiter has claimed.
    inbox: VecDeque<Value>,
    /// **Arrival order**, every message, with the instant it landed.
    ///
    /// Separate from `inbox` on purpose. The inbox is a work queue that
    /// waiters shuffle; this is the tape. A transcript has to record what the
    /// server actually said in the order it said it, including the messages a
    /// waiter stepped over on its way to the one it wanted — those are the
    /// ones that catch a server publishing diagnostics it should not.
    log: Vec<(Value, Instant)>,
    ended: bool,
    framing_error: Option<String>,
    workspace: PathBuf,
    encoding: Encoding,
    next_id: i64,
    timeout: Duration,
}

impl Session {
    /// Spawn `<bin> lsp` with `workspace` as its working directory.
    ///
    /// The cwd matters: the server resolves module graphs from the entry
    /// file's directory, and a session run from the repo root would resolve
    /// `resolve/two_mod/main.lu`'s siblings differently from one run in the
    /// samples tree.
    pub fn spawn(bin: &Path, workspace: &Path) -> Result<Self, Error> {
        Self::spawn_with_env(bin, workspace, &[])
    }

    /// Spawn with extra environment variables.
    ///
    /// The only current use is the compiler's own injected-slow-query knob
    /// (`WOLF_QUERY_TEST_SLOW_MS`), which is what makes a cancellation
    /// scenario *deterministic* rather than a race the harness usually loses:
    /// against a query that answers in two milliseconds, a `$/cancelRequest`
    /// sent immediately after the request almost always arrives too late, and a
    /// transcript recording "cancelled" would fail on a fast machine. Slowing
    /// the query is the difference between testing cancellation and testing the
    /// scheduler.
    ///
    /// It is spawn-time state, so it lives in the **script** — the committed
    /// input — and not in the transcript, which is derived output.
    pub fn spawn_with_env(
        bin: &Path,
        workspace: &Path,
        env: &[(String, String)],
    ) -> Result<Self, Error> {
        let mut command = Command::new(bin);
        command
            .arg("lsp")
            .current_dir(workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in env {
            command.env(k, v);
        }
        let mut child = command.spawn()?;

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        let stdin = child.stdin.take().expect("stdin was piped");

        let (tx, rx) = channel();
        let reader = std::thread::spawn(move || read_loop(BufReader::new(stdout), &tx));

        let buffer = Arc::new(Mutex::new(String::new()));
        let sink = Arc::clone(&buffer);
        // Drained continuously and forever: a server whose stderr nobody reads
        // deadlocks at the pipe buffer, and the deadlock looks like a hang in
        // the *client*, which is how it stays unfixed for months.
        let stderr_pump = std::thread::spawn(move || {
            use std::io::Read;
            let mut r = stderr;
            let mut chunk = [0u8; 4096];
            loop {
                match r.read(&mut chunk) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => {
                        let mut s = sink.lock().unwrap_or_else(|e| e.into_inner());
                        s.push_str(&String::from_utf8_lossy(&chunk[..n]));
                    }
                }
            }
        });

        Ok(Self {
            child,
            stdin: Some(stdin),
            rx,
            reader: Some(reader),
            stderr_pump: Some(stderr_pump),
            stderr: buffer,
            inbox: VecDeque::new(),
            log: Vec::new(),
            ended: false,
            framing_error: None,
            workspace: workspace.to_path_buf(),
            encoding: Encoding::Utf16,
            next_id: 1,
            timeout: DEFAULT_TIMEOUT,
        })
    }

    /// The workspace this session runs in.
    #[must_use]
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    /// The encoding negotiated at `initialize`. `utf-16` before then, which is
    /// what the protocol says an un-negotiated session uses.
    #[must_use]
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Override the per-message wait.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Everything the server has written to stderr so far.
    #[must_use]
    pub fn stderr_text(&self) -> String {
        self.stderr
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Allocate the next request id. Integers only — report 09's hygiene list
    /// records that at least one tier-0 client cannot parse a string id.
    pub fn next_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ------------------------------------------------------------ send --

    /// Send a well-formed message.
    pub fn send(&mut self, msg: &Value) -> Result<(), Error> {
        let body = serde_json::to_vec(msg).expect("a Value always serializes");
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| Error::Protocol("stdin already closed".to_string()))?;
        framing::write_frame(stdin, &body).map_err(Error::Io)
    }

    /// Send bytes verbatim — framing included, and nothing validated.
    ///
    /// The reason [`crate::framing`] is ours rather than a dependency: the
    /// malformed-frame cases need to lie about `Content-Length`, split a
    /// header, or omit the blank line.
    pub fn send_raw(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| Error::Protocol("stdin already closed".to_string()))?;
        framing::write_raw(stdin, bytes).map_err(Error::Io)
    }

    /// Close stdin. A client that goes away without `exit` must not leave the
    /// server running (§6's orphan check).
    pub fn close_stdin(&mut self) {
        self.stdin = None;
    }

    /// Send a request and return its id.
    pub fn request(&mut self, method: &str, params: Value) -> Result<i64, Error> {
        let id = self.next_id();
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        Ok(id)
    }

    /// Send a notification.
    pub fn notify(&mut self, method: &str, params: Value) -> Result<(), Error> {
        self.send(&json!({"jsonrpc": "2.0", "method": method, "params": params}))
    }

    // ------------------------------------------------------------ recv --

    /// Pull one message from the inbox or the wire, whichever has one.
    ///
    /// `Ok(None)` means the deadline passed with nothing new; the caller
    /// decides whether that is a timeout or the end of a drain.
    pub fn pump(&mut self, deadline: Instant) -> Result<Option<Value>, Error> {
        if let Some(msg) = self.inbox.pop_front() {
            return Ok(Some(msg));
        }
        self.pump_wire(deadline)
    }

    fn pump_wire(&mut self, deadline: Instant) -> Result<Option<Value>, Error> {
        if self.ended {
            return Ok(None);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        match self.rx.recv_timeout(remaining) {
            Ok(Incoming::Message(v)) => {
                self.log.push((v.clone(), Instant::now()));
                Ok(Some(v))
            }
            Ok(Incoming::End(err)) => {
                self.ended = true;
                self.framing_error = err;
                Ok(None)
            }
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => {
                self.ended = true;
                Ok(None)
            }
        }
    }

    /// Wait for the first message satisfying `want`, holding everything else
    /// in the inbox in arrival order.
    ///
    /// Returns the message and the wall time from `since` to its arrival —
    /// which is what `lspconf bench` records and what `t_us` carries.
    pub fn wait_for(
        &mut self,
        awaited: &str,
        since: Instant,
        timeout: Duration,
        mut want: impl FnMut(&Value) -> bool,
    ) -> Result<(Value, Duration), Error> {
        // The inbox first: the answer may already be here.
        if let Some(i) = self.inbox.iter().position(&mut want) {
            let msg = self.inbox.remove(i).expect("index came from position");
            return Ok((msg, since.elapsed()));
        }
        let deadline = Instant::now() + timeout;
        loop {
            match self.pump_wire(deadline)? {
                Some(msg) if want(&msg) => return Ok((msg, since.elapsed())),
                Some(msg) => self.inbox.push_back(msg),
                None if self.ended => {
                    return Err(Error::ServerGone {
                        awaited: awaited.to_string(),
                        stderr: self.stderr_text(),
                    });
                }
                None => {
                    return Err(Error::Timeout {
                        awaited: awaited.to_string(),
                        seen: self.trace(),
                        stderr: self.stderr_text(),
                    });
                }
            }
        }
    }

    /// Wait for the response to `id` — `result` or `error`, both are answers.
    pub fn response(&mut self, id: i64, since: Instant) -> Result<(Value, Duration), Error> {
        let timeout = self.timeout;
        self.wait_for(&format!("the response to id {id}"), since, timeout, |m| {
            m.get("id").and_then(Value::as_i64) == Some(id)
                && (m.get("result").is_some() || m.get("error").is_some())
        })
    }

    /// Wait for a notification, optionally constrained to one document.
    pub fn notification(
        &mut self,
        method: &str,
        uri: Option<&str>,
        since: Instant,
    ) -> Result<(Value, Duration), Error> {
        let timeout = self.timeout;
        let want_uri = uri.map(str::to_string);
        let awaited = match &want_uri {
            Some(u) => format!("a `{method}` notification for {u}"),
            None => format!("a `{method}` notification"),
        };
        self.wait_for(&awaited, since, timeout, |m| {
            if m.get("method").and_then(Value::as_str) != Some(method) {
                return false;
            }
            match &want_uri {
                None => true,
                Some(u) => m.pointer("/params/uri").and_then(Value::as_str) == Some(u.as_str()),
            }
        })
    }

    /// Collect everything that arrives within `window`, then stop.
    ///
    /// Used where the claim is *absence* — "no second `publishDiagnostics`
    /// after a `didChange` that changed nothing" — which cannot be waited for,
    /// only waited out.
    pub fn drain(&mut self, window: Duration) -> Result<Vec<Value>, Error> {
        let deadline = Instant::now() + window;
        let mut out: Vec<Value> = self.inbox.drain(..).collect();
        while Instant::now() < deadline {
            match self.pump_wire(deadline)? {
                Some(msg) => out.push(msg),
                None => break,
            }
        }
        Ok(out)
    }

    /// Everything received this session, rendered one line each.
    #[must_use]
    pub fn trace(&self) -> Vec<String> {
        let mut out: Vec<String> = self.log.iter().map(|(v, _)| render(v)).collect();
        if let Some(e) = &self.framing_error {
            out.push(format!("<framing error: {e}>"));
        }
        out
    }

    /// How many messages have arrived. A cursor into [`Session::log_from`].
    #[must_use]
    pub fn log_len(&self) -> usize {
        self.log.len()
    }

    /// Messages that arrived at or after cursor `from`, in **arrival order**,
    /// each with the instant it landed.
    #[must_use]
    pub fn log_from(&self, from: usize) -> &[(Value, Instant)] {
        &self.log[from.min(self.log.len())..]
    }

    /// The framing-level reason the stream ended, if it ended badly.
    #[must_use]
    pub fn framing_error(&self) -> Option<&str> {
        self.framing_error.as_deref()
    }

    // ------------------------------------------------------- lifecycle --

    /// Drive `initialize` + `initialized` with a client-capability document,
    /// record the negotiated encoding, and return the `initialize` result.
    pub fn initialize(&mut self, capabilities: &Value) -> Result<Value, Error> {
        let started = Instant::now();
        let root = crate::slash_path(&self.workspace);
        let id = self.request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": file_uri(&self.workspace),
                "workspaceFolders": [{"uri": file_uri(&self.workspace), "name": "samples"}],
                "capabilities": capabilities,
                "clientInfo": {"name": "lspconf", "version": env!("CARGO_PKG_VERSION")},
            }),
        )?;
        let _ = root;
        let (resp, _) = self.response(id, started)?;
        if let Some(err) = resp.get("error") {
            return Err(Error::Protocol(format!("initialize failed: {err}")));
        }
        self.encoding = resp
            .pointer("/result/capabilities/positionEncoding")
            .and_then(Value::as_str)
            .and_then(|s| s.parse().ok())
            // The protocol's default when the server says nothing.
            .unwrap_or(Encoding::Utf16);
        self.notify("initialized", json!({}))?;
        Ok(resp)
    }

    /// `shutdown` then `exit`, and wait for the process to leave.
    ///
    /// Returns the exit code. A server that ignores `exit` is a server every
    /// editor leaks a process for, so the wait is bounded and a survivor is
    /// killed and reported rather than left behind.
    pub fn shutdown_exit(&mut self) -> Result<Option<i32>, Error> {
        let started = Instant::now();
        let id = self.request("shutdown", Value::Null)?;
        let _ = self.response(id, started)?;
        self.notify("exit", Value::Null)?;
        self.close_stdin();
        self.wait_exit(EXIT_TIMEOUT)
    }

    /// Wait for the process to exit, killing it if it overstays.
    pub fn wait_exit(&mut self, within: Duration) -> Result<Option<i32>, Error> {
        let deadline = Instant::now() + within;
        loop {
            match self.child.try_wait()? {
                Some(status) => return Ok(status.code()),
                None if Instant::now() >= deadline => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return Err(Error::Protocol(format!(
                        "the server was still running {within:?} after `exit` — killed. \
                         An editor that closes leaks this process."
                    )));
                }
                None => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    }

    /// Is the process still alive?
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// The child's OS process id, for the orphan checks.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Unconditional: a test that fails mid-session must not leave a server
        // holding the terminal, and every wait above is already bounded.
        self.stdin = None;
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
        if let Some(h) = self.stderr_pump.take() {
            let _ = h.join();
        }
    }
}

fn read_loop(stdout: BufReader<std::process::ChildStdout>, tx: &Sender<Incoming>) {
    let mut reader = FrameReader::new(stdout);
    loop {
        match reader.read_frame() {
            Ok(Some(body)) => match serde_json::from_slice::<Value>(&body) {
                Ok(v) => {
                    if tx.send(Incoming::Message(v)).is_err() {
                        return;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Incoming::End(Some(format!(
                        "the server sent a frame that is not JSON: {e}"
                    ))));
                    return;
                }
            },
            Ok(None) => {
                let _ = tx.send(Incoming::End(None));
                return;
            }
            Err(e) => {
                let _ = tx.send(Incoming::End(Some(e.to_string())));
                return;
            }
        }
    }
}

/// One line describing a message, for failure reports.
fn render(v: &Value) -> String {
    let id = v.get("id").map_or(String::new(), |i| format!(" id={i}"));
    if let Some(m) = v.get("method").and_then(Value::as_str) {
        let uri = v
            .pointer("/params/uri")
            .or_else(|| v.pointer("/params/textDocument/uri"))
            .and_then(Value::as_str)
            .map_or(String::new(), |u| format!(" {}", short_uri(u)));
        return format!("{m}{id}{uri}");
    }
    if let Some(e) = v.get("error") {
        let code = e.get("code").map_or(String::new(), ToString::to_string);
        return format!("error{id} code={code}");
    }
    let shape = match v.get("result") {
        Some(Value::Null) => "null".to_string(),
        Some(Value::Array(a)) => format!("[{} item(s)]", a.len()),
        Some(Value::Object(_)) => "{…}".to_string(),
        Some(other) => other.to_string(),
        None => "<no result>".to_string(),
    };
    format!("response{id} result={shape}")
}

fn short_uri(uri: &str) -> String {
    uri.rsplit('/').next().unwrap_or(uri).to_string()
}

/// A path as a `file://` URI.
///
/// Hand-rolled rather than pulled in as a dependency, and percent-encoding is
/// conservative — the server compares URI strings byte-for-byte and echoes
/// back exactly what it was sent (report 09's hygiene list), so the *shape*
/// this function emits is part of what the suite asserts.
#[must_use]
pub fn file_uri(path: &Path) -> String {
    let slashed = crate::slash_path(path);
    let mut out = String::from("file://");
    // A Windows path starts `C:/…` with no leading slash; the URI needs one.
    if !slashed.starts_with('/') {
        out.push('/');
    }
    for byte in slashed.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(byte as char);
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// The filesystem path a `file://` URI names, or `None` if it is not one.
#[must_use]
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let mut decoded = Vec::new();
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            if let Ok(b) = u8::from_str_radix(hex, 16) {
                decoded.push(b);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    let text = String::from_utf8(decoded).ok()?;
    // `file:///C:/x` → `C:/x` on Windows; `file:///a/b` → `/a/b` elsewhere.
    let text = if cfg!(windows) {
        text.strip_prefix('/').unwrap_or(&text).to_string()
    } else {
        text
    };
    Some(PathBuf::from(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris_round_trip() {
        let p = if cfg!(windows) {
            PathBuf::from("C:\\work\\samples\\hello.lu")
        } else {
            PathBuf::from("/work/samples/hello.lu")
        };
        let uri = file_uri(&p);
        assert!(uri.starts_with("file:///"), "{uri}");
        assert_eq!(
            uri_to_path(&uri).unwrap(),
            PathBuf::from(crate::slash_path(&p))
        );
    }

    #[test]
    fn a_space_is_percent_encoded_and_survives_the_round_trip() {
        // platform-shaped like the round-trip test above: the windows
        // decode branch strips the leading slash for drive-letter form
        let p = if cfg!(windows) {
            PathBuf::from("C:\\work\\my samples\\a.lu")
        } else {
            PathBuf::from("/work/my samples/a.lu")
        };
        let uri = file_uri(&p);
        assert!(uri.contains("%20"), "{uri}");
        assert_eq!(
            uri_to_path(&uri).unwrap(),
            PathBuf::from(crate::slash_path(&p))
        );
    }

    #[test]
    fn non_file_uris_are_refused_rather_than_guessed() {
        assert!(uri_to_path("untitled:Untitled-1").is_none());
    }

    #[test]
    fn render_names_the_method_and_the_document() {
        let v = json!({"method": "textDocument/publishDiagnostics",
                       "params": {"uri": "file:///w/hello.lu", "diagnostics": []}});
        assert_eq!(render(&v), "textDocument/publishDiagnostics hello.lu");
    }

    #[test]
    fn render_distinguishes_an_error_response_from_a_result() {
        let ok = json!({"id": 3, "result": []});
        let err = json!({"id": 4, "error": {"code": -32601, "message": "no"}});
        assert_eq!(render(&ok), "response id=3 result=[0 item(s)]");
        assert_eq!(render(&err), "error id=4 code=-32601");
    }
}
