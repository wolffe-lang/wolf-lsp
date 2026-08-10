//! The transcript record model — the on-disk artifact ls01 replays.
//!
//! Frozen at ls00 (sprint §2). A transcript is JSONL: one message per line,
//! the first line a [`Header`], every later line a [`Record`]. Nothing in
//! this module talks to a server; the format is deliberately buildable and
//! testable before `wolf lsp` exists (D34 — the server is the compiler).

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use crate::matcher::Matcher;
use crate::normalize::Stage;

/// First line of every transcript.
///
/// `wolf_pin` is the sha from `vendor/upstream/PIN`. A transcript recorded
/// against one pin and replayed against another is a reviewable event, not a
/// silent one: ls01 refuses the replay rather than guessing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    /// Format version. Bumps are their own commit with a migration note.
    pub transcript: u32,
    /// `<client>/<scenario>`, e.g. `fackr/open-hover`.
    pub name: String,
    /// wolf-lang commit the session was recorded against.
    pub wolf_pin: String,
    /// Capability profile name under `profiles/` (ls01 §4).
    pub profile: String,
    /// Workspace root the session ran in, repo-relative.
    pub workspace: String,
    /// ISO-8601 date the session was recorded.
    pub recorded: String,
}

/// Message direction. `c2s` records are driven; `s2c` records are expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dir {
    /// Client to server — the harness sends these.
    C2s,
    /// Server to client — the harness waits for and matches these.
    S2c,
}

/// JSON-RPC message shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Has an `id` and expects a response.
    Request,
    /// Carries `result` or `error` for an `id`.
    Response,
    /// Has no `id` and expects nothing.
    Notification,
}

/// One recorded message.
///
/// `t_us` is a **sidecar**: it is written by `lspconf record` and read only by
/// `lspconf bench`. Comparison never reads it — a transcript that fails
/// because the server got faster is a harness bug (sprint §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// Ordinal within the transcript, 1-based, gapless.
    pub seq: u32,
    pub dir: Dir,
    pub kind: Kind,
    /// Request/response correlation id. Renumbered by the `ids` normalization
    /// stage, so the recorded value is a label, not a promise.
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<Value>,
    /// Method name; absent on responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub params: Option<Value>,
    /// Response payload. `Some(Value::Null)` and `None` are **different**: the
    /// former is `"result":null`, which is what `shutdown` actually returns,
    /// and the latter is a record with no result at all (an error response).
    /// Serde's stock `Option` impl folds both to `None`, which would make the
    /// codec lossy and `shutdown` unrepresentable — hence `present_value`.
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub result: Option<Value>,
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub error: Option<Value>,
    /// Per-record matcher override. Absent means "use the default for this
    /// method" ([`crate::defaults`]) — the stability contract lives in that
    /// table, and this field is how a transcript opts out of it.
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub matcher: Option<Matcher>,
    /// Normalization stages applied before comparison, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub normalize: Vec<Stage>,
    /// Recorded wall time in microseconds. Sidecar only; never compared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t_us: Option<u64>,
}

impl Record {
    /// The subtree a relative matcher path resolves against.
    ///
    /// A matcher written `set:diagnostics` means "the `diagnostics` array of
    /// this message's payload", and what counts as the payload depends on the
    /// message: `result` for a response, `params` otherwise, `error` for an
    /// error response. Writing `set:/params/diagnostics` in a transcript would
    /// couple the assertion to the JSON-RPC envelope for no benefit.
    #[must_use]
    pub fn payload(&self) -> &Value {
        const NULL: &Value = &Value::Null;
        match self.kind {
            Kind::Response => self.result.as_ref().or(self.error.as_ref()).unwrap_or(NULL),
            Kind::Request | Kind::Notification => self.params.as_ref().unwrap_or(NULL),
        }
    }

    /// Mutable counterpart of [`Record::payload`], for normalization.
    pub fn payload_mut(&mut self) -> Option<&mut Value> {
        match self.kind {
            Kind::Response => self.result.as_mut().or(self.error.as_mut()),
            Kind::Request | Kind::Notification => self.params.as_mut(),
        }
    }

    /// Every JSON value in the record that normalization may rewrite.
    ///
    /// `id` is included because the `ids` stage renumbers it; `t_us` is not,
    /// because nothing may normalize a value nothing compares.
    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        [
            self.id.as_mut(),
            self.params.as_mut(),
            self.result.as_mut(),
            self.error.as_mut(),
        ]
        .into_iter()
        .flatten()
    }

    /// The matcher that governs this record, defaulting per method.
    ///
    /// A **response** record carries no method of its own — the method is the
    /// one on the request that shares its id — so callers holding a whole
    /// transcript should use [`Transcript::matcher_for`], which correlates.
    /// `method` here is that correlated name; `None` falls back to the
    /// record's own, which is right for requests and notifications.
    #[must_use]
    pub fn effective_matcher(&self, method: Option<&str>) -> Matcher {
        if let Some(explicit) = self.matcher.clone() {
            return explicit;
        }
        // An ERROR response is not the method's result shape.
        //
        // The defaults table is keyed by method, and rightly so — but it
        // answers "how do I compare a `documentSymbol` result", and a
        // `documentSymbol` that *failed* carries `{code, message}` instead.
        // Handing that object to `set:` produces "needs an array in the
        // transcript" on two identical payloads, which is the worst kind of
        // failure: a mismatch report about a difference that is not there.
        //
        // Every error response therefore defaults to [`Matcher::Subset`],
        // which compares exactly what the claim is — the JSON-RPC `code` the
        // server refused with — plus the `message` beside it. Pinning that
        // prose is deliberate and cheap: these strings are the shim's own
        // constants, and a rewording is one `lspconf record` away from a
        // reviewed one-line diff.
        if self.kind == Kind::Response && self.result.is_none() && self.error.is_some() {
            return Matcher::Subset;
        }
        crate::defaults::for_method(method.or(self.method.as_deref()), self.kind)
    }
}

/// Deserialize a field that is *present*, preserving an explicit `null`.
///
/// `#[serde(default)]` supplies `None` when the key is missing; this supplies
/// `Some(Value::Null)` when the key is there and null. Without it the two are
/// indistinguishable and the JSONL codec is not a fixed point — a `"result":null`
/// line comes back as a record with no result and re-serializes without it.
fn present_value<'de, D: Deserializer<'de>>(de: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(de).map(Some)
}

/// A parsed transcript: header plus records.
#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    pub header: Header,
    pub records: Vec<Record>,
}

impl Transcript {
    /// The method a record belongs to, correlating responses back to their
    /// request by `id`.
    ///
    /// Ids are only unique among *outstanding* requests, so the search runs
    /// backwards from the response: the matching request is the most recent
    /// one with that id, which is the one it answers.
    #[must_use]
    pub fn method_for(&self, index: usize) -> Option<&str> {
        let rec = self.records.get(index)?;
        if let Some(m) = rec.method.as_deref() {
            return Some(m);
        }
        let id = rec.id.as_ref()?;
        self.records[..index]
            .iter()
            .rev()
            .find(|r| r.kind == Kind::Request && r.id.as_ref() == Some(id))
            .and_then(|r| r.method.as_deref())
    }

    /// The matcher governing the record at `index`, with response methods
    /// correlated. Panics-free: an out-of-range index yields the fallback.
    #[must_use]
    pub fn matcher_for(&self, index: usize) -> Matcher {
        let method = self.method_for(index);
        self.records
            .get(index)
            .map_or(Matcher::Subset, |r| r.effective_matcher(method))
    }

    /// Structural checks that do not need a server.
    ///
    /// Run by `lspconf verify`, which is the server-free half of CI (sprint
    /// §3): a transcript can be wrong long before anything replays it.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errs = Vec::new();
        if self.header.transcript != crate::FORMAT_VERSION {
            errs.push(format!(
                "header: transcript format {} is not the supported version {}",
                self.header.transcript,
                crate::FORMAT_VERSION
            ));
        }
        if self.header.name.is_empty() {
            errs.push("header: name is empty".to_string());
        }
        if self.header.wolf_pin.len() != 40
            || !self.header.wolf_pin.chars().all(|c| c.is_ascii_hexdigit())
        {
            errs.push(format!(
                "header: wolf_pin `{}` is not a 40-char hex sha",
                self.header.wolf_pin
            ));
        }
        for (i, rec) in self.records.iter().enumerate() {
            let want = u32::try_from(i + 1).unwrap_or(u32::MAX);
            if rec.seq != want {
                errs.push(format!(
                    "record {}: seq is {}, expected {want}",
                    i + 1,
                    rec.seq
                ));
            }
            match rec.kind {
                Kind::Request if rec.id.is_none() => {
                    errs.push(format!("record {}: request without id", rec.seq));
                }
                Kind::Response if rec.id.is_none() => {
                    errs.push(format!("record {}: response without id", rec.seq));
                }
                Kind::Notification if rec.id.is_some() => {
                    errs.push(format!("record {}: notification carries an id", rec.seq));
                }
                Kind::Response if rec.result.is_none() && rec.error.is_none() => {
                    errs.push(format!(
                        "record {}: response has neither result nor error",
                        rec.seq
                    ));
                }
                Kind::Response if rec.result.is_some() && rec.error.is_some() => {
                    errs.push(format!(
                        "record {}: response has both result and error",
                        rec.seq
                    ));
                }
                Kind::Request | Kind::Notification if rec.method.is_none() => {
                    errs.push(format!("record {}: {:?} without method", rec.seq, rec.kind));
                }
                _ => {}
            }
            if rec.dir == Dir::C2s && rec.matcher.is_some() {
                errs.push(format!(
                    "record {}: c2s records are sent, not matched — `match` is meaningless here",
                    rec.seq
                ));
            }
        }
        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }
}

impl fmt::Display for Dir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Dir::C2s => "c2s",
            Dir::S2c => "s2c",
        })
    }
}

/// Sort every object key in a JSON value, in place, recursively.
///
/// `serde_json` without `preserve_order` already stores objects in a
/// `BTreeMap`, so this is a no-op on values that round-tripped through it —
/// but it is not a no-op on values constructed by hand, and canonical output
/// is a load-bearing property (sprint §2: a re-record must produce a
/// reviewable diff, not a reshuffle). Keeping it explicit means the guarantee
/// survives someone enabling that feature for an unrelated reason.
pub fn sort_keys(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut sorted = Map::new();
            let taken = std::mem::take(map);
            let mut entries: Vec<(String, Value)> = taken.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (k, mut v) in entries {
                sort_keys(&mut v);
                sorted.insert(k, v);
            }
            *map = sorted;
        }
        Value::Array(items) => {
            for item in items {
                sort_keys(item);
            }
        }
        _ => {}
    }
}
