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
    /// Rewrite absolute workspace paths to `$WS` and temp dirs to `$TMP`,
    /// inside plain strings and inside `file://` URIs alike. Separators are
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
             ids, paths, pid, uri, version, serverinfo, nulls",
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
            tmp: std::env::temp_dir(),
            ids: BTreeMap::new(),
            next_id: 1,
        }
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
                    let (ws, tmp) = (self.workspace.clone(), self.tmp.clone());
                    for v in rec.values_mut() {
                        map_strings(v, &mut |s| elide_paths(s, ws.as_deref(), &tmp));
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
fn elide_paths(s: &str, workspace: Option<&Path>, tmp: &Path) -> String {
    let folded = s.replace('\\', "/");
    let mut out = folded;
    for (root, placeholder) in [(workspace, WS), (Some(tmp), TMP)]
        .into_iter()
        .filter_map(|(p, ph)| p.map(|p| (p, ph)))
    {
        let root = root.to_string_lossy().replace('\\', "/");
        let root = root.trim_end_matches('/');
        if root.is_empty() {
            continue;
        }
        out = out.replace(root, placeholder);
    }
    out
}
