//! Normalization — the stages that run before any matcher looks at a value.
//!
//! The whole point (sprint §2, ls01 §3): a transcript must fail when the
//! server's *behavior* changes and must not fail when its *incidental output*
//! changes. Ids, absolute paths, and pids differ on every run and on every
//! machine; if they reach the matcher, the suite is abandoned within two
//! sprints.
//!
//! ## Unconditional vs opt-in
//!
//! A stage is unconditional only when **no assertion could ever legitimately
//! depend on the value it elides**. That is true of request ids (renumbered,
//! so correlation survives but the numbering does not), of absolute paths
//! (machine-specific by construction), and of process ids. Everything else is
//! named per record in `normalize`, because eliding it might be throwing away
//! the claim: `version` is the obvious trap — on `publishDiagnostics` it is an
//! echo of what the client sent, but ls01 §6 asserts version *discipline*
//! against a stale `didChange`, and that transcript needs the raw number.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::record::{Record, Transcript};

/// Workspace placeholder. Chosen to be un-typeable as a real path component
/// so a normalization that silently did nothing is visible in a diff.
pub const WS: &str = "$WS";
/// Temp-directory placeholder.
pub const TMP: &str = "$TMP";
/// Repository-root placeholder.
///
/// Distinct from [`WS`] because a client is entitled to pick a root ABOVE the
/// directory the session ran in, and which one it picked is data worth keeping.
/// helix walks up for `wolf.pkg` then `.git`; eglot asks project.el, which finds
/// the git root. Both therefore report the repository root while opening a file
/// under `vendor/upstream/samples`, and collapsing that to `$WS` would erase the
/// difference between a client that scoped itself to the workspace and one that
/// did not.
pub const REPO: &str = "$REPO";
/// Placeholder for an elided `version` member.
pub const VERSION: &str = "$VERSION";
/// Placeholder for a fully-elided URI.
pub const URI: &str = "$URI";

/// One normalization stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Renumber request/response ids by first appearance, 1-based.
    /// Unconditional.
    Ids,
    /// Rewrite absolute workspace paths to `$WS`, repository roots to `$REPO`
    /// and temp dirs to `$TMP`,
    /// inside plain strings and inside `file://` URIs alike — and, since
    /// le06, inside object KEYS as well, which is where
    /// `WorkspaceEdit.changes` keeps its document URIs. Separators are
    /// normalized to `/` first, so a Windows run and a Linux run produce the
    /// same transcript. Unconditional.
    Paths,
    /// Replace process ids with `0`. Unconditional.
    Pid,
    /// Collapse whole `file://` URIs to `$URI`. Opt-in: for records where
    /// *which* document was involved is not the claim being made.
    Uri,
    /// Replace every member named `version` with `$VERSION`. Opt-in — see the
    /// module note; this is not the same thing as [`Stage::ServerInfo`].
    Version,
    /// Drop `serverInfo` entirely. Opt-in: the server's own version string
    /// changes on every release and pins nothing.
    ServerInfo,
    /// Drop object members whose value is explicit `null`, so a client that
    /// sends `{"x": null}` and one that omits `x` compare equal — LSP's own
    /// optionality rule. Opt-in, because a few methods do distinguish them.
    Nulls,
    /// Sort the arrays LSP leaves unordered — `diagnostics` and
    /// `relatedInformation` — by `(uri, range, code, message)`, at any depth.
    ///
    /// The `set:` matcher already compares those arrays as multisets, so this
    /// stage changes no verdict. What it changes is the **snapshot**: a
    /// normalized view whose diagnostic order depends on which worker thread
    /// finished first churns for no reason, and a churning snapshot stops
    /// being read. Opt-in, because `documentSymbol` nesting order *is*
    /// behavior and must never be sorted away.
    DiagSort,
}

/// Stages every record gets whether or not it asks.
pub const UNCONDITIONAL: &[Stage] = &[Stage::Ids, Stage::Paths, Stage::Pid];

impl Stage {
    fn as_str(self) -> &'static str {
        match self {
            Stage::Ids => "ids",
            Stage::Paths => "paths",
            Stage::Pid => "pid",
            Stage::Uri => "uri",
            Stage::Version => "version",
            Stage::ServerInfo => "serverinfo",
            Stage::Nulls => "nulls",
            Stage::DiagSort => "diagsort",
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Failure to parse a stage name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseStageError(pub String);

impl fmt::Display for ParseStageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown normalization stage `{}` — expected one of: \
             ids, paths, pid, uri, version, serverinfo, nulls, diagsort",
            self.0
        )
    }
}

impl std::error::Error for ParseStageError {}

impl FromStr for Stage {
    type Err = ParseStageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "ids" => Stage::Ids,
            "paths" => Stage::Paths,
            "pid" => Stage::Pid,
            "uri" => Stage::Uri,
            "version" => Stage::Version,
            "serverinfo" => Stage::ServerInfo,
            "nulls" => Stage::Nulls,
            "diagsort" => Stage::DiagSort,
            other => return Err(ParseStageError(other.to_string())),
        })
    }
}

impl Serialize for Stage {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Stage {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        s.parse().map_err(de::Error::custom)
    }
}

/// Applies stages across a message stream, carrying the state that spans
/// records (the id renumbering table).
///
/// One `Normalizer` per stream: the recorded side and the live side are
/// normalized independently and then compared, which is what makes id
/// renumbering meaningful — both sides collapse to "first id seen is 1".
#[derive(Debug, Clone)]
pub struct Normalizer {
    workspace: Option<PathBuf>,
    repo_root: Option<PathBuf>,
    tmp: PathBuf,
    ids: BTreeMap<String, u64>,
    next_id: u64,
}

impl Normalizer {
    /// `workspace` is the absolute path the session ran in; without it, the
    /// `paths` stage still elides temp dirs but cannot produce `$WS`.
    #[must_use]
    pub fn new(workspace: Option<PathBuf>) -> Self {
        Self {
            workspace,
            repo_root: None,
            tmp: std::env::temp_dir(),
            ids: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Also elide `root`, the repository root, as `$REPO`.
    ///
    /// Needed because a client may report a root ABOVE the workspace: helix's
    /// `roots = ["wolf.pkg", ".git"]` and eglot's project.el both resolve to the
    /// repository root while the session runs in `vendor/upstream/samples`. That
    /// path is machine-specific and would otherwise survive into the committed
    /// transcript, which
    /// `client_recorded::captured_client_messages_carry_no_absolute_paths`
    /// exists to catch — and did.
    #[must_use]
    pub fn with_repo_root(mut self, root: PathBuf) -> Self {
        self.repo_root = Some(root);
        self
    }

    /// Normalize a whole transcript in place, in `seq` order.
    pub fn run(&mut self, transcript: &mut Transcript) {
        for rec in &mut transcript.records {
            self.record(rec);
        }
    }

    /// Normalize one record in place: unconditional stages plus whatever the
    /// record names, deduplicated and applied in a fixed order so the result
    /// does not depend on how the author happened to write the list.
    pub fn record(&mut self, rec: &mut Record) {
        let mut stages: Vec<Stage> = UNCONDITIONAL.to_vec();
        stages.extend(rec.normalize.iter().copied());
        stages.sort_unstable();
        stages.dedup();

        for stage in stages {
            match stage {
                Stage::Ids => self.renumber_id(rec),
                Stage::Paths => {
                    let (ws, repo, tmp) = (
                        self.workspace.clone(),
                        self.repo_root.clone(),
                        self.tmp.clone(),
                    );
                    for v in rec.values_mut() {
                        // KEYS TOO — see `map_strings_and_keys`. This is the
                        // one stage that walks them, and the one that has to.
                        map_strings_and_keys(v, &mut |s| {
                            elide_paths(s, ws.as_deref(), repo.as_deref(), &tmp)
                        });
                    }
                }
                Stage::Pid => {
                    for v in rec.values_mut() {
                        elide_member(v, "processId", Value::from(0));
                        elide_member(v, "pid", Value::from(0));
                    }
                }
                Stage::Uri => {
                    for v in rec.values_mut() {
                        map_strings(v, &mut |s| {
                            if s.starts_with("file://") {
                                URI.to_string()
                            } else {
                                s.to_string()
                            }
                        });
                    }
                }
                Stage::Version => {
                    for v in rec.values_mut() {
                        elide_member(v, "version", Value::from(VERSION));
                    }
                }
                Stage::ServerInfo => {
                    for v in rec.values_mut() {
                        drop_member(v, "serverInfo");
                    }
                }
                Stage::Nulls => {
                    for v in rec.values_mut() {
                        drop_nulls(v);
                    }
                }
                Stage::DiagSort => {
                    for v in rec.values_mut() {
                        sort_unordered(v);
                    }
                }
            }
        }
    }

    fn renumber_id(&mut self, rec: &mut Record) {
        let Some(id) = rec.id.as_ref() else { return };
        if matches!(id, Value::String(s) if s == VERSION) {
            return;
        }
        let key = id.to_string();
        let next = &mut self.next_id;
        let renumbered = *self.ids.entry(key).or_insert_with(|| {
            let n = *next;
            *next += 1;
            n
        });
        rec.id = Some(Value::from(renumbered));
    }
}

/// Rewrite every string in a JSON value.
/// [`map_strings`], plus the object KEYS.
///
/// LSP has exactly one map whose keys are data rather than field names, and it
/// is the one that broke: `WorkspaceEdit.changes` is
/// `{ [uri: DocumentUri]: TextEdit[] }`. So a `rename` answer to a client that
/// does NOT declare `workspaceEdit.documentChanges` — nvim and helix among the
/// maintained profiles — carries its document URIs as keys and nowhere else,
/// and every stage that walked only values left them **absolute**. Measured on
/// this branch before the fix, at the c97b81c… tree: six transcripts shipped a
/// developer's home directory, including
/// `encoding/astral-navigate-*.jsonl`'s `file:///Users/<name>/…/fixtures/
/// astral.lu`. A checkout under a different name replays those and fails on a
/// path, which is the exact class of failure `Stage::Paths` is unconditional
/// to prevent.
///
/// Walking keys is safe rather than merely expedient, and by construction:
/// every OTHER member name in the protocol is a fixed identifier — `range`,
/// `newText`, `uri` — with no separator, no drive letter and no `file://`
/// prefix in it, so `elide_paths` is the identity on all of them. Only a
/// prefix is ever replaced, so two distinct keys stay distinct and no entry
/// can be lost.
///
/// `Stage::Uri` deliberately does NOT use this: it collapses a whole URI to
/// one placeholder, so applying it to keys would merge every entry of a
/// multi-file `changes` map into a single `$URI` and destroy the record. A
/// transcript that needs both wants `documentChanges`, where the URI is a
/// value.
fn map_strings_and_keys(value: &mut Value, f: &mut impl FnMut(&str) -> String) {
    match value {
        Value::String(s) => *s = f(s),
        Value::Array(items) => {
            for item in items {
                map_strings_and_keys(item, f);
            }
        }
        Value::Object(map) => {
            let taken = std::mem::take(map);
            for (k, mut v) in taken {
                map_strings_and_keys(&mut v, f);
                map.insert(f(&k), v);
            }
        }
        _ => {}
    }
}

fn map_strings(value: &mut Value, f: &mut impl FnMut(&str) -> String) {
    match value {
        Value::String(s) => *s = f(s),
        Value::Array(items) => {
            for item in items {
                map_strings(item, f);
            }
        }
        Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                map_strings(v, f);
            }
        }
        _ => {}
    }
}

/// Replace every occurrence of a named member with `to`, at any depth.
fn elide_member(value: &mut Value, name: &str, to: Value) {
    match value {
        Value::Object(map) => {
            if let Some(slot) = map.get_mut(name) {
                *slot = to.clone();
            }
            for (_, v) in map.iter_mut() {
                elide_member(v, name, to.clone());
            }
        }
        Value::Array(items) => {
            for item in items {
                elide_member(item, name, to.clone());
            }
        }
        _ => {}
    }
}

/// Remove every occurrence of a named member, at any depth.
fn drop_member(value: &mut Value, name: &str) {
    match value {
        Value::Object(map) => {
            map.remove(name);
            for (_, v) in map.iter_mut() {
                drop_member(v, name);
            }
        }
        Value::Array(items) => {
            for item in items {
                drop_member(item, name);
            }
        }
        _ => {}
    }
}

/// Arrays whose order the protocol leaves free, and which this stage sorts.
const UNORDERED_ARRAYS: &[&str] = &["diagnostics", "relatedInformation"];

/// Sort every unordered array in the value, at any depth.
fn sort_unordered(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, v) in map.iter_mut() {
                sort_unordered(v);
                if UNORDERED_ARRAYS.contains(&key.as_str())
                    && let Value::Array(items) = v
                {
                    items.sort_by_cached_key(diagnostic_sort_key);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                sort_unordered(item);
            }
        }
        _ => {}
    }
}

/// `(uri, range, code, message)` — the sprint's identity tuple, rendered as a
/// sortable string.
///
/// Every component is fixed-width-padded rather than compared numerically,
/// because the key has to be *one* orderable value and a diagnostic on line 2
/// must sort before one on line 10. A tuple of `Value`s would need an `Ord`
/// impl on `Value`, which `serde_json` deliberately does not provide.
fn diagnostic_sort_key(item: &Value) -> String {
    let s = |ptr: &str| -> String {
        item.pointer(ptr)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let n = |ptr: &str| -> String {
        format!(
            "{:010}",
            item.pointer(ptr).and_then(Value::as_u64).unwrap_or(0)
        )
    };
    // `code` is a string or a number depending on the server; render either.
    let code = match item.get("code") {
        Some(Value::String(c)) => c.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    };
    // Related information nests the range under `location`; a diagnostic does
    // not. Concatenating both keeps one key function for both array kinds.
    [
        s("/location/uri"),
        n("/range/start/line"),
        n("/range/start/character"),
        n("/range/end/line"),
        n("/range/end/character"),
        n("/location/range/start/line"),
        n("/location/range/start/character"),
        code,
        s("/message"),
    ]
    .join("\u{1f}")
}

fn drop_nulls(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|_, v| !v.is_null());
            for (_, v) in map.iter_mut() {
                drop_nulls(v);
            }
        }
        Value::Array(items) => {
            for item in items {
                drop_nulls(item);
            }
        }
        _ => {}
    }
}

/// Rewrite machine-specific path prefixes inside an arbitrary string.
///
/// Separators are folded to `/` **before** matching, because the same
/// workspace is `C:\…\samples` on the Windows runner and `/…/samples`
/// elsewhere, and LSP positions are byte offsets — a transcript that
/// disagrees about separators disagrees about every column downstream.
/// The workspace is elided FIRST and the repository root second: the workspace
/// is the deeper path, so a URI inside it must collapse to `$WS/...` rather than
/// to `$REPO/vendor/upstream/samples/...`. Reversing the order would make every
/// existing transcript's document URIs change shape.
fn elide_paths(s: &str, workspace: Option<&Path>, repo_root: Option<&Path>, tmp: &Path) -> String {
    let folded = s.replace('\\', "/");
    let mut out = folded;
    let home = home_dir();
    for (root, placeholder) in [(workspace, WS), (repo_root, REPO), (Some(tmp), TMP)]
        .into_iter()
        .filter_map(|(p, ph)| p.map(|p| (p, ph)))
    {
        let root = root.to_string_lossy().replace('\\', "/");
        let root = root.trim_end_matches('/');
        if root.is_empty() {
            continue;
        }
        // THE URI FORM FIRST, AND IT IS ONLY DISTINCT ON WINDOWS.
        //
        // A `file:` URI needs three slashes before an absolute path, and on
        // unix the workspace root supplies the third itself: `/Users/…` is
        // both the path and the URI tail, so `file:///Users/…/samples/x`
        // elides to `file://$WS/x` and every transcript in the library is
        // written that way. A Windows root is `D:/a/…` with no leading slash,
        // so the live URI is `file:///D:/a/…/samples/x` and eliding only the
        // plain form would leave `file:///$WS/x` — one slash more than every
        // recorded transcript, and a mismatch on the URI of every
        // `publishDiagnostics` in the library.
        //
        // So the slash-prefixed root is elided FIRST, to the same placeholder.
        // On unix the two strings are identical and this is a no-op; on
        // Windows it is what makes a transcript recorded on one platform
        // comparable on the other, which is the entire promise of this stage
        // (`two_machines_normalize_to_the_same_transcript`).
        //
        // Measured on the first windows-latest `server-lane` run to get past
        // the URI-expansion half of this bug (le06): 59 transcripts, three
        // records each, every one `/uri: expected "file://$WS/…", got
        // "file:///$WS/…"`.
        if !root.starts_with('/') {
            out = out.replace(&format!("/{root}"), placeholder);
        }
        out = out.replace(root, placeholder);
        // THE TILDE FORM, which is a real spelling and not a courtesy. eglot
        // names its workspace folder with emacs's `abbreviate-file-name`, so
        // `transcripts/emacs/smoke.jsonl` records
        // `workspaceFolders[0].name = "~/…/wolf-lsp/"` — a home directory in a
        // committed artifact that no amount of absolute-prefix matching can
        // see, because the absolute prefix is not in the string.
        if let Some(home) = &home
            && let Some(rest) = root.strip_prefix(home.as_str())
            && !rest.is_empty()
        {
            out = out.replace(&format!("~{rest}"), placeholder);
        }
    }
    out
}

/// The home directory, `/`-folded, or `None` when the environment names none.
///
/// Read here rather than passed in because it is the same kind of fact as the
/// separator: a property of the machine that recorded, needed only to un-say
/// it. A run with no `HOME` (and no `USERPROFILE`) simply elides no tilde
/// form, which is the pre-le06 behavior.
fn home_dir() -> Option<String> {
    let raw = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let folded = raw.replace('\\', "/");
    let trimmed = folded.trim_end_matches('/');
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
