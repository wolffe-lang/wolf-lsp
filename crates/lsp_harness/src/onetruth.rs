//! `lspconf onetruth` — D34 made falsifiable.
//!
//! "The compiler *is* the language server" is a slogan until something
//! executes it. This module runs both surfaces over the same bytes and asserts
//! they say the same thing:
//!
//! - `wolf conform-run <file> --error-format=json`, whose stderr carries one
//!   `diag_schema: 1` object per diagnostic with **byte spans**;
//! - an LSP session that opens the same file unmodified and waits for
//!   `publishDiagnostics`, whose ranges are **positions in the negotiated
//!   encoding**.
//!
//! The byte→position transform is [`lsp_transcript::encoding`] — this repo's
//! own, not the server's — so a divergence here is a real divergence and not
//! the server agreeing with itself.
//!
//! Message text is compared **exactly**, and only here. Everywhere else in the
//! suite prose is a regex, because D22 owns diagnostic wording upstream and
//! this repo must not become a second review gate on it. Here the wording *is*
//! the claim: the editor shows what the build shows, down to the sentence.
//!
//! # A mismatch is a wolf-lang bug
//!
//! Not something to normalize away, not something to patch around here. The
//! interpreter track's divergence-filing rule applies: the editor layer detects
//! divergence, it does not hide it. [`Divergence::filing`] renders the report
//! that goes on the upstream issue, with both records attached.
//!
//! # Two comparisons, one of which is not yet positional
//!
//! A `diag_schema: 1` line identifies a span's file by **integer index**. Index
//! 0 is the entry file, so a diagnostic whose primary span lands there is
//! compared *positionally* — code, severity, span, message, related
//! information.
//!
//! Mirroring the server's own filter and comparing only index 0 would make this
//! check agree by construction — and it would hide the exact failure it exists
//! to find. wolf's module rule is D32: every `.lu` in a directory is ONE module,
//! so `wolf build` on an entry reports diagnostics whose primary span is in a
//! *sibling*, and an editor that never publishes those shows a clean file for a
//! package that does not build. That is not hypothetical: it is what this check
//! found (DIV-LSP-001, fixed upstream in `7117882`).
//!
//! So there are two comparisons:
//!
//! - **positional** — entry-file diagnostics, compared in full. Spans included;
//!   this is the one the encoding suite feeds into.
//! - **reachability** — every build diagnostic, wherever its primary lands,
//!   must be published to *some* document of the module. Compared by
//!   `(code, severity, message)`.
//!
//! [`SECONDARY_FILE_TABLE_REQUEST`] records why the second one is weaker than
//! the first, and that the blocker has since been removed: pin `70bdd35` grants
//! the request with an additive `files` member (index → path, SourceMap intern
//! order), so a sibling-primary diagnostic can now be resolved to a path and
//! compared positionally against the publish for *that* URI. Doing so is an
//! ls01 change, not an ls03 one, and it is owed — until it lands, a
//! sibling-primary diagnostic published to the right document at the *wrong*
//! range still passes reachability.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use lsp_transcript::encoding::{self, Encoding, LineIndex, Position};
use serde_json::Value;

use crate::profiles::Profile;
use crate::session::{self, Session, file_uri};

/// One entry of `divergences.toml` — a divergence that has been *filed*.
///
/// Filing is not forgiving. An entry that matches nothing fails the check just
/// as loudly as an unfiled divergence, because a ledger that outlives its bug
/// is how a fixed bug gets re-introduced without anyone noticing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filed {
    pub id: String,
    pub sample: String,
    pub code: String,
    /// `unreachable`, `only_build`, or `only_editor`.
    pub kind: String,
    /// The upstream issue. `pending` is allowed only until it is opened.
    pub filed: String,
}

impl Filed {
    /// Does this entry account for `code` diverging on `sample` in `kind`?
    #[must_use]
    pub fn covers(&self, sample: &str, code: &str, kind: &str) -> bool {
        self.sample == sample && self.code == code && self.kind == kind
    }
}

/// Read `divergences.toml`.
///
/// Hand-rolled for the same reason as `samples.toml`: a five-key manifest does
/// not justify a TOML dependency in a repo whose whole posture is dependency
/// thinness, and the ledger is short by design.
pub fn load_ledger(repo_root: &Path) -> Result<Vec<Filed>, Error> {
    let path = repo_root.join("divergences.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let mut out: Vec<Filed> = Vec::new();
    let mut current: Option<Filed> = None;
    let mut in_block = false;
    for raw in text.lines() {
        let line = raw.trim();
        if in_block {
            // A `"""` block value: skipped wholesale. `summary` is prose for a
            // human reading the issue, and nothing here matches on it.
            if line.ends_with("\"\"\"") {
                in_block = false;
            }
            continue;
        }
        if line == "[[divergence]]" {
            if let Some(d) = current.take() {
                out.push(d);
            }
            current = Some(Filed {
                id: String::new(),
                sample: String::new(),
                code: String::new(),
                kind: String::new(),
                filed: String::new(),
            });
            continue;
        }
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        if value == "\"\"\"" {
            in_block = true;
            continue;
        }
        let value = value.trim_matches('"').to_string();
        if let Some(d) = current.as_mut() {
            match key {
                "id" => d.id = value,
                "sample" => d.sample = value,
                "code" => d.code = value,
                "kind" => d.kind = value,
                "filed" => d.filed = value,
                "summary" => {}
                other => {
                    return Err(Error::Cli(format!(
                        "divergences.toml: unknown key `{other}` — a typo that silently did \
                         nothing is how a ledger stops meaning anything"
                    )));
                }
            }
        }
    }
    if let Some(d) = current.take() {
        out.push(d);
    }
    for d in &out {
        for (name, value) in [
            ("id", &d.id),
            ("sample", &d.sample),
            ("code", &d.code),
            ("kind", &d.kind),
            ("filed", &d.filed),
        ] {
            if value.is_empty() {
                return Err(Error::Cli(format!(
                    "divergences.toml: an entry is missing `{name}`"
                )));
            }
        }
    }
    Ok(out)
}

/// Split a divergence into the part the ledger accounts for and the part it
/// does not.
///
/// Returns `(known, unknown)` as `(id, description)` pairs, plus the ledger
/// entries that matched — so a caller can report an entry that matched nothing.
#[must_use]
pub fn triage<'a>(
    divergence: &Divergence,
    ledger: &'a [Filed],
) -> (Vec<(&'a Filed, String)>, Vec<String>) {
    let mut known = Vec::new();
    let mut unknown = Vec::new();
    let mut classify = |code: &str, kind: &str, text: String| match ledger
        .iter()
        .find(|f| f.covers(&divergence.sample, code, kind))
    {
        Some(f) => known.push((f, text)),
        None => unknown.push(text),
    };
    for c in &divergence.only_build {
        classify(&c.code, "only_build", format!("only in the build: {c}"));
    }
    for c in &divergence.only_editor {
        classify(&c.code, "only_editor", format!("only in the editor: {c}"));
    }
    for i in &divergence.unreachable {
        classify(
            &i.code,
            "unreachable",
            format!("published to no document: {i}"),
        );
    }
    (known, unknown)
}

/// The upstream request this check was waiting on — **granted** at pin
/// `70bdd35`, and kept here as the record of what to build next.
pub const SECONDARY_FILE_TABLE_REQUEST: &str = "\
wolf-lang: `diag_schema` v1 identified a span's file by integer index with no \
index→path table in the output, so a consumer could not resolve a span in a \
file other than the entry. Requested: a `files` member (index → path) on the \
conform-run record. GRANTED in `7117882` (additive within schema 1, so older \
consumers are unaffected). Owed here: teach the reachability comparison to use \
it, which makes sibling-primary diagnostics positional like entry ones.";

/// One diagnostic, reduced to what both surfaces can be asked about.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Claim {
    pub code: String,
    pub severity: u8,
    pub start: (u32, u32),
    pub end: (u32, u32),
    pub message: String,
    /// Related-information messages, sorted. Ranges are excluded on purpose —
    /// see the module note on the missing file table.
    pub related: Vec<String>,
}

impl std::fmt::Display for Claim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} sev{} @{}:{}-{}:{} {:?}",
            self.code,
            self.severity,
            self.start.0,
            self.start.1,
            self.end.0,
            self.end.1,
            first_line(&self.message)
        )
    }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("")
}

/// A diagnostic reduced to what the reachability comparison can see.
///
/// No span: a build diagnostic outside the entry file has no resolvable file,
/// so its byte offsets cannot be converted. Identity is what remains, and
/// identity is enough to answer "does the editor ever show this at all".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identity {
    pub code: String,
    pub severity: u8,
    pub message: String,
}

impl std::fmt::Display for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} sev{} {:?}",
            self.code,
            self.severity,
            first_line(&self.message)
        )
    }
}

impl From<&Claim> for Identity {
    fn from(c: &Claim) -> Self {
        Self {
            code: c.code.clone(),
            severity: c.severity,
            message: c.message.clone(),
        }
    }
}

/// The result for one sample.
#[derive(Debug, Clone)]
pub struct Divergence {
    pub sample: String,
    pub encoding: Encoding,
    /// In the build but not in the editor, compared positionally within the
    /// entry file.
    pub only_build: Vec<Claim>,
    /// In the editor but not in the build, same comparison.
    pub only_editor: Vec<Claim>,
    /// Build diagnostics the editor never publishes to ANY document of the
    /// module. The strongest finding this check produces: `wolf build` fails
    /// and the editor shows a clean file.
    pub unreachable: Vec<Identity>,
}

impl Divergence {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.only_build.is_empty() && self.only_editor.is_empty() && self.unreachable.is_empty()
    }

    /// The body of the upstream issue. Both records attached, per §9.
    #[must_use]
    pub fn filing(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "wolf-lang: `wolf build` and `wolf lsp` disagree on {} (positionEncoding {})\n\n",
            self.sample, self.encoding
        ));
        out.push_str(
            "D34 says one binary, one truth. These two surfaces answered differently \
             over identical bytes.\n\n",
        );
        if !self.only_build.is_empty() {
            out.push_str("## Reported by `wolf conform-run`, absent from `publishDiagnostics`\n\n");
            for c in &self.only_build {
                out.push_str(&format!("- {c}\n"));
            }
            out.push('\n');
        }
        if !self.only_editor.is_empty() {
            out.push_str("## Reported by `publishDiagnostics`, absent from `wolf conform-run`\n\n");
            for c in &self.only_editor {
                out.push_str(&format!("- {c}\n"));
            }
            out.push('\n');
        }
        if !self.unreachable.is_empty() {
            out.push_str(
                "## Reported by `wolf conform-run`, published to NO document of the module\n\n\
                 Every `.lu` in the directory was opened (D32: one directory is one module) \
                 and none of them received these. A user editing this package sees a clean \
                 file for code that does not build.\n\n",
            );
            for c in &self.unreachable {
                out.push_str(&format!("- {c}\n"));
            }
            out.push('\n');
        }
        out.push_str(
            "Positions were converted from the build's byte spans with an \
             implementation independent of the server's (`lsp_transcript::encoding`), \
             so a conversion bug in the shim shows up here rather than cancelling out.\n",
        );
        out
    }
}

/// Anything that stops the check from running.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Session(session::Error),
    Cli(String),
    Profile(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Session(e) => write!(f, "{e}"),
            Error::Cli(m) | Error::Profile(m) => write!(f, "{m}"),
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

/// Run the check for one sample under one profile.
///
/// `sample` is workspace-relative, exactly as `samples.toml` lists it.
pub fn check(
    repo_root: &Path,
    bin: &Path,
    workspace: &Path,
    sample: &str,
    profile: &Profile,
) -> Result<Divergence, Error> {
    let path = workspace.join(sample);
    let bytes = std::fs::read(&path)?;
    let index = LineIndex::new(&bytes);

    let mut session = Session::spawn(bin, workspace)?;
    session.initialize(&profile.capabilities)?;
    let enc = session.encoding();

    // The entry document, compared positionally.
    let uri = file_uri(&path);
    let started = Instant::now();
    open(&mut session, &uri, &bytes)?;
    let (published, _) =
        session.notification("textDocument/publishDiagnostics", Some(&uri), started)?;
    let editor = editor_claims(&published);

    // Every sibling in the same directory, for reachability. D32 makes them
    // one module, so this is what an editor with the project open would show.
    let mut reachable: Vec<Identity> = editor.iter().map(Identity::from).collect();
    for sibling in siblings(&path)? {
        let sib_bytes = std::fs::read(&sibling)?;
        let sib_uri = file_uri(&sibling);
        let started = Instant::now();
        open(&mut session, &sib_uri, &sib_bytes)?;
        let (published, _) =
            session.notification("textDocument/publishDiagnostics", Some(&sib_uri), started)?;
        reachable.extend(editor_claims(&published).iter().map(Identity::from));
    }
    session.shutdown_exit()?;

    let build = build_claims(bin, workspace, sample, &bytes, &index, enc)?;
    let build_all = build_identities(bin, workspace, sample)?;

    let _ = repo_root;
    let mut divergence = diff(sample, enc, &build, &editor);
    divergence.unreachable = missing(&build_all, &reachable);
    Ok(divergence)
}

fn open(session: &mut Session, uri: &str, bytes: &[u8]) -> Result<(), Error> {
    session.notify(
        "textDocument/didOpen",
        serde_json::json!({"textDocument": {
            "uri": uri, "languageId": "wolf", "version": 1,
            "text": String::from_utf8_lossy(bytes)}}),
    )?;
    Ok(())
}

/// The other `.lu` files in the same directory — the rest of the module (D32).
fn siblings(entry: &Path) -> Result<Vec<std::path::PathBuf>, Error> {
    let Some(dir) = entry.parent() else {
        return Ok(Vec::new());
    };
    let mut out: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p != entry && p.extension().is_some_and(|e| e == "lu"))
        .collect();
    // read_dir order is platform noise.
    out.sort();
    Ok(out)
}

/// Multiset difference: what is in `build` and not in `editor`.
fn missing(build: &[Identity], editor: &[Identity]) -> Vec<Identity> {
    let mut have: BTreeMap<&Identity, usize> = BTreeMap::new();
    for i in editor {
        *have.entry(i).or_default() += 1;
    }
    let mut out = Vec::new();
    for i in build {
        match have.get_mut(i) {
            Some(n) if *n > 0 => *n -= 1,
            _ => out.push(i.clone()),
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Every diagnostic the build reports, whatever file its primary lands in.
pub fn build_identities(
    bin: &Path,
    workspace: &Path,
    sample: &str,
) -> Result<Vec<Identity>, Error> {
    // Positions are irrelevant here and the file is unknown, so any encoding
    // and any buffer will do for the conversion this reuses.
    let claims = raw_claims(bin, workspace, sample)?;
    Ok(claims)
}

/// The set difference, both ways, as multisets.
///
/// Ordering is not compared (§9 says so); presence, identity, and position are.
#[must_use]
pub fn diff(sample: &str, enc: Encoding, build: &[Claim], editor: &[Claim]) -> Divergence {
    let mut left: BTreeMap<&Claim, usize> = BTreeMap::new();
    for c in build {
        *left.entry(c).or_default() += 1;
    }
    let mut right: BTreeMap<&Claim, usize> = BTreeMap::new();
    for c in editor {
        *right.entry(c).or_default() += 1;
    }
    let mut only_build = Vec::new();
    let mut only_editor = Vec::new();
    for (c, n) in &left {
        let m = right.get(*c).copied().unwrap_or(0);
        for _ in m..*n {
            only_build.push((*c).clone());
        }
    }
    for (c, n) in &right {
        let m = left.get(*c).copied().unwrap_or(0);
        for _ in m..*n {
            only_editor.push((*c).clone());
        }
    }
    Divergence {
        sample: sample.to_string(),
        encoding: enc,
        only_build,
        only_editor,
        unreachable: Vec::new(),
    }
}

/// Parse `publishDiagnostics` params into claims.
#[must_use]
pub fn editor_claims(published: &Value) -> Vec<Claim> {
    published
        .pointer("/params/diagnostics")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(editor_claim).collect())
        .unwrap_or_default()
}

fn editor_claim(d: &Value) -> Claim {
    let pos = |ptr: &str| -> (u32, u32) {
        (
            d.pointer(&format!("{ptr}/line"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            d.pointer(&format!("{ptr}/character"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
        )
    };
    let mut related: Vec<String> = d
        .get("relatedInformation")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|r| {
                    r.get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default();
    related.sort();
    Claim {
        code: match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => String::new(),
        },
        severity: d.get("severity").and_then(Value::as_u64).unwrap_or(0) as u8,
        start: pos("/range/start"),
        end: pos("/range/end"),
        message: d
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        related,
    }
}

/// Every `diag_schema` line, reduced to identity, with no file filter.
fn raw_claims(bin: &Path, workspace: &Path, sample: &str) -> Result<Vec<Identity>, Error> {
    let empty: &[u8] = &[];
    let index = LineIndex::new(empty);
    let mut out = Vec::new();
    for v in diag_lines(bin, workspace, sample)? {
        let c = build_claim(&v, empty, &index, Encoding::Utf8);
        out.push(Identity::from(&c));
    }
    Ok(out)
}

/// The `diag_schema` objects on `conform-run`'s stderr.
fn diag_lines(bin: &Path, workspace: &Path, sample: &str) -> Result<Vec<Value>, Error> {
    let out = Command::new(bin)
        .args(["conform-run", sample, "--error-format=json"])
        .current_dir(workspace)
        .output()?;
    // A nonzero status is normal — a file that fails to compile is the point.
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut lines = Vec::new();
    for line in stderr.lines() {
        let line = line.trim();
        if !line.starts_with('{') {
            continue;
        }
        let v: Value = serde_json::from_str(line).map_err(|e| {
            Error::Cli(format!(
                "conform-run emitted a stderr line that is not JSON: {e}"
            ))
        })?;
        if v.get("diag_schema").is_some() {
            lines.push(v);
        }
    }
    Ok(lines)
}

/// Run `wolf conform-run` and turn its stderr into claims, in the entry file.
pub fn build_claims(
    bin: &Path,
    workspace: &Path,
    sample: &str,
    bytes: &[u8],
    index: &LineIndex,
    enc: Encoding,
) -> Result<Vec<Claim>, Error> {
    let mut claims = Vec::new();
    for v in diag_lines(bin, workspace, sample)? {
        // Only diagnostics whose *primary* span is in the entry file can be
        // compared positionally — index 0 is the entry and nothing in the
        // output names the others. The ones skipped here are not dropped: they
        // go through the reachability comparison instead.
        if v.pointer("/primary/file").and_then(Value::as_u64) != Some(0) {
            continue;
        }
        claims.push(build_claim(&v, bytes, index, enc));
    }
    Ok(claims)
}

fn build_claim(v: &Value, bytes: &[u8], index: &LineIndex, enc: Encoding) -> Claim {
    let span = |ptr: &str| -> (u32, u32) {
        let a = v.pointer(ptr).and_then(Value::as_array);
        let get = |i: usize| {
            a.and_then(|a| a.get(i))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32
        };
        (get(0), get(1))
    };
    let (lo, hi) = span("/primary/span");
    let (start, end) = encoding::span_to_range(bytes, index, lo, hi, enc);
    let to_pair = |p: Position| (p.line, p.character);

    // The shim appends the primary label and every note to `message`, so that
    // clients rendering only `message` still show them (report 09: fackr's
    // gutter dot is where D22's work goes to die). Reconstructing the same
    // string is what makes "the editor shows what the build shows" testable.
    let mut message = v
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let label = v
        .pointer("/primary/label")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !label.is_empty() {
        message.push('\n');
        message.push_str(label);
    }
    for note in v
        .get("notes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        message.push_str("\nnote: ");
        message.push_str(note);
    }

    let mut related: Vec<String> = v
        .get("secondary")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|s| {
                    s.get("label")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default();
    related.sort();

    Claim {
        code: v
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        severity: match v.get("severity").and_then(Value::as_str) {
            Some("error") => 1,
            Some("warning") => 2,
            Some("information") => 3,
            Some("hint") => 4,
            _ => 0,
        },
        start: to_pair(start),
        end: to_pair(end),
        message,
        related,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn claim(code: &str, start: (u32, u32)) -> Claim {
        Claim {
            code: code.to_string(),
            severity: 1,
            start,
            end: (start.0, start.1 + 1),
            message: "m".to_string(),
            related: Vec::new(),
        }
    }

    #[test]
    fn identical_sets_diverge_in_neither_direction() {
        let a = vec![claim("E0001", (1, 2)), claim("E0002", (3, 4))];
        // Order is explicitly not compared.
        let b = vec![claim("E0002", (3, 4)), claim("E0001", (1, 2))];
        assert!(diff("s", Encoding::Utf16, &a, &b).is_empty());
    }

    #[test]
    fn a_span_that_moved_by_one_column_is_a_divergence_in_both_directions() {
        // The exact failure the encoding conversion causes: same code, same
        // line, one column off. It must not cancel out.
        let a = vec![claim("E0003", (5, 14))];
        let b = vec![claim("E0003", (5, 15))];
        let d = diff("s", Encoding::Utf16, &a, &b);
        assert_eq!(d.only_build.len(), 1);
        assert_eq!(d.only_editor.len(), 1);
        assert!(d.filing().contains("E0003"));
    }

    #[test]
    fn duplicates_are_counted_rather_than_deduplicated() {
        let a = vec![claim("E0001", (1, 1)), claim("E0001", (1, 1))];
        let b = vec![claim("E0001", (1, 1))];
        let d = diff("s", Encoding::Utf16, &a, &b);
        assert_eq!(d.only_build.len(), 1);
        assert!(d.only_editor.is_empty());
    }

    #[test]
    fn the_build_message_is_reconstructed_the_way_the_shim_builds_it() {
        let src = b"fn main() {}\n";
        let index = LineIndex::new(src);
        let v = json!({
            "diag_schema": 1, "code": "E0501", "severity": "error",
            "message": "the bounds on `T` do not provide `Show`",
            "primary": {"file": 0, "span": [3, 7], "label": "`T` could be any type here"},
            "secondary": [], "notes": ["generic bodies are checked once"], "suggestions": []
        });
        let c = build_claim(&v, src, &index, Encoding::Utf16);
        assert_eq!(
            c.message,
            "the bounds on `T` do not provide `Show`\n\
             `T` could be any type here\n\
             note: generic bodies are checked once"
        );
        assert_eq!(c.severity, 1);
        assert_eq!(c.start, (0, 3));
        assert_eq!(c.end, (0, 7));
    }

    #[test]
    fn a_build_span_after_astral_text_converts_to_a_different_column_per_encoding() {
        // One line, one emoji, then the span. This is the whole §9 ↔ §5 join:
        // if the shim and this file disagree about the transform, the check
        // reports a divergence that is really an encoding bug.
        let src = "let x = \"\u{1F43A}\" + y\n".as_bytes();
        let index = LineIndex::new(src);
        let lo = src.len() as u32 - 2; // `y`
        let v = json!({
            "diag_schema": 1, "code": "E0000", "severity": "error", "message": "m",
            "primary": {"file": 0, "span": [lo, lo + 1], "label": ""},
            "secondary": [], "notes": [], "suggestions": []
        });
        let u8c = build_claim(&v, src, &index, Encoding::Utf8).start.1;
        let u16c = build_claim(&v, src, &index, Encoding::Utf16).start.1;
        let u32c = build_claim(&v, src, &index, Encoding::Utf32).start.1;
        // Nine ASCII, then one emoji, then four more ASCII before `y`:
        // 9+4+4 bytes, 9+2+4 UTF-16 units, 9+1+4 code points.
        assert_eq!((u8c, u16c, u32c), (17, 15, 14));
    }

    #[test]
    fn editor_claims_read_the_shape_the_server_actually_sends() {
        let published = json!({"params": {"uri": "file:///w/a.lu", "diagnostics": [{
            "code": "E0501", "severity": 1, "message": "m",
            "range": {"start": {"line": 13, "character": 14},
                      "end": {"line": 13, "character": 15}},
            "relatedInformation": [
                {"message": "b", "location": {"uri": "file:///w/a.lu"}},
                {"message": "a", "location": {"uri": "file:///w/a.lu"}}],
            "source": "wolf"}]}});
        let c = &editor_claims(&published)[0];
        assert_eq!(c.code, "E0501");
        assert_eq!(c.start, (13, 14));
        // Sorted: related information order is not a claim either surface makes.
        assert_eq!(c.related, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn a_clean_file_produces_no_claims_on_either_side() {
        let published = json!({"params": {"uri": "file:///w/a.lu", "diagnostics": []}});
        assert!(editor_claims(&published).is_empty());
        assert!(diff("s", Encoding::Utf8, &[], &[]).is_empty());
    }
}
