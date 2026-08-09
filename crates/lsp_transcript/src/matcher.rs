//! The matcher engine — how a recorded message is compared to a live one.
//!
//! **The matcher is declared in the transcript, not in harness code** (sprint
//! §2). That is deliberate: the comparison policy for a message is a property
//! of the scenario being asserted, and burying it in Rust means a reviewer
//! reading the transcript cannot see what is actually being claimed.
//!
//! The defaults ([`crate::defaults`]) follow what LSP actually pins. The
//! protocol explicitly permits a server to grow capabilities and leaves many
//! array orders unspecified, so the default is *structural*; byte-equality is
//! opt-in for the few places the protocol really does fix an answer.

use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::pointer::{self, Pointer};

/// How to compare one expected `s2c` record against what arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matcher {
    /// Deep equality after normalization. Use where the protocol pins the
    /// answer: ranges, codes, severities, formatting output.
    Exact,
    /// The recorded value must be *contained in* what arrived: every recorded
    /// object key must be present and match, extra keys in the live message
    /// are fine. This is the forward-compatibility matcher — a server that
    /// gains a capability must not turn the suite red.
    Subset,
    /// Order-insensitive array at a payload-relative pointer; everything
    /// outside that array is compared as [`Matcher::Subset`]. Elements are
    /// compared exactly, because order is what LSP leaves free — element
    /// content is behavior.
    Set(Pointer),
    /// The string at a payload-relative pointer is a regex in the transcript
    /// and must match the live string; everything else is
    /// [`Matcher::Subset`]. For human-readable prose, whose wording D22 owns
    /// upstream — this repo must not become a second review gate on it.
    Regex(Pointer),
    /// Always matches. For incidental traffic (`window/logMessage`,
    /// `$/progress`) that carries no claim.
    Ignore,
}

/// A comparison failure, located.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    /// JSON pointer into the payload, or `""` for the payload root.
    pub path: String,
    pub reason: String,
}

impl Mismatch {
    fn at(path: &str, reason: impl Into<String>) -> Self {
        Self {
            path: path.to_string(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let where_ = if self.path.is_empty() {
            "<payload>"
        } else {
            &self.path
        };
        write!(f, "{where_}: {}", self.reason)
    }
}

impl Matcher {
    /// Compare an expected payload against a live one.
    ///
    /// Both are assumed already normalized ([`crate::normalize`]); this
    /// function does no eliding of its own, so a failure here is a difference
    /// that survived normalization — which is exactly the definition of a
    /// behavior change.
    pub fn compare(&self, expected: &Value, actual: &Value) -> Result<(), Mismatch> {
        match self {
            Matcher::Ignore => Ok(()),
            Matcher::Exact => compare_exact(expected, actual, ""),
            Matcher::Subset => compare_subset(expected, actual, ""),
            Matcher::Set(ptr) => {
                let (e_arr, e_rest) = split_at(expected, ptr)?;
                let (a_arr, a_rest) = split_at(actual, ptr)?;
                compare_subset(&e_rest, &a_rest, "")?;
                compare_multiset(&e_arr, &a_arr, &ptr.to_string())
            }
            Matcher::Regex(ptr) => {
                let e_str = pointer::resolve(expected, ptr).ok_or_else(|| {
                    Mismatch::at(&ptr.to_string(), "no such member in the transcript")
                })?;
                let a_str = pointer::resolve(actual, ptr).ok_or_else(|| {
                    Mismatch::at(&ptr.to_string(), "no such member in the live message")
                })?;
                let pattern = e_str.as_str().ok_or_else(|| {
                    Mismatch::at(
                        &ptr.to_string(),
                        "regex matcher needs a string in the transcript",
                    )
                })?;
                let live = a_str.as_str().ok_or_else(|| {
                    Mismatch::at(
                        &ptr.to_string(),
                        "regex matcher needs a string in the live message",
                    )
                })?;
                let re = regex::Regex::new(pattern).map_err(|e| {
                    Mismatch::at(&ptr.to_string(), format!("invalid regex `{pattern}`: {e}"))
                })?;
                if !re.is_match(live) {
                    return Err(Mismatch::at(
                        &ptr.to_string(),
                        format!("`{live}` does not match /{pattern}/"),
                    ));
                }
                // The rest of the message still has to hold up; blank the
                // string out on both sides so subset does not re-compare it.
                let (_, e_rest) = split_at(expected, ptr)?;
                let (_, a_rest) = split_at(actual, ptr)?;
                compare_subset(&e_rest, &a_rest, "")
            }
        }
    }
}

/// Remove the value at `ptr`, returning `(removed, remainder)`.
///
/// A missing member yields `Null`, which the callers turn into a located
/// error rather than a silent pass.
fn split_at(value: &Value, ptr: &Pointer) -> Result<(Value, Value), Mismatch> {
    let mut rest = value.clone();
    let taken = pointer::take(&mut rest, ptr);
    Ok((taken.unwrap_or(Value::Null), rest))
}

fn compare_exact(expected: &Value, actual: &Value, path: &str) -> Result<(), Mismatch> {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            for (k, ev) in e {
                let child = format!("{path}/{}", pointer::escape(k));
                let av = a
                    .get(k)
                    .ok_or_else(|| Mismatch::at(&child, "missing from the live message"))?;
                compare_exact(ev, av, &child)?;
            }
            for k in a.keys() {
                if !e.contains_key(k) {
                    return Err(Mismatch::at(
                        &format!("{path}/{}", pointer::escape(k)),
                        "present in the live message but not in the transcript (exact match)",
                    ));
                }
            }
            Ok(())
        }
        (Value::Array(e), Value::Array(a)) => {
            if e.len() != a.len() {
                return Err(Mismatch::at(
                    path,
                    format!(
                        "array length {} in the transcript, {} live",
                        e.len(),
                        a.len()
                    ),
                ));
            }
            for (i, (ev, av)) in e.iter().zip(a).enumerate() {
                compare_exact(ev, av, &format!("{path}/{i}"))?;
            }
            Ok(())
        }
        (e, a) if e == a => Ok(()),
        (e, a) => Err(Mismatch::at(path, format!("expected {e}, got {a}"))),
    }
}

fn compare_subset(expected: &Value, actual: &Value, path: &str) -> Result<(), Mismatch> {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            for (k, ev) in e {
                let child = format!("{path}/{}", pointer::escape(k));
                match a.get(k) {
                    Some(av) => compare_subset(ev, av, &child)?,
                    // LSP treats an absent optional and an explicit null as
                    // the same thing in most places, and clients disagree
                    // about which they send. Recording `null` must not
                    // demand the key exist.
                    None if ev.is_null() => {}
                    None => return Err(Mismatch::at(&child, "missing from the live message")),
                }
            }
            Ok(())
        }
        (Value::Array(e), Value::Array(a)) => {
            if e.len() != a.len() {
                return Err(Mismatch::at(
                    path,
                    format!(
                        "array length {} in the transcript, {} live \
                         (subset descends arrays elementwise; use `set:` if order is free)",
                        e.len(),
                        a.len()
                    ),
                ));
            }
            for (i, (ev, av)) in e.iter().zip(a).enumerate() {
                compare_subset(ev, av, &format!("{path}/{i}"))?;
            }
            Ok(())
        }
        (e, a) if e == a => Ok(()),
        (e, a) => Err(Mismatch::at(path, format!("expected {e}, got {a}"))),
    }
}

/// Multiset comparison: same elements, any order, duplicates counted.
fn compare_multiset(expected: &Value, actual: &Value, path: &str) -> Result<(), Mismatch> {
    let e = expected
        .as_array()
        .ok_or_else(|| Mismatch::at(path, "`set:` matcher needs an array in the transcript"))?;
    let a = actual
        .as_array()
        .ok_or_else(|| Mismatch::at(path, "`set:` matcher needs an array in the live message"))?;
    if e.len() != a.len() {
        return Err(Mismatch::at(
            path,
            format!("{} element(s) in the transcript, {} live", e.len(), a.len()),
        ));
    }
    let mut used: HashSet<usize> = HashSet::new();
    for (i, ev) in e.iter().enumerate() {
        let hit = a
            .iter()
            .enumerate()
            .find(|(j, av)| !used.contains(j) && *av == ev);
        match hit {
            Some((j, _)) => {
                used.insert(j);
            }
            None => {
                return Err(Mismatch::at(
                    &format!("{path}/{i}"),
                    format!("no unmatched live element equals {ev}"),
                ));
            }
        }
    }
    Ok(())
}

impl fmt::Display for Matcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Matcher::Exact => f.write_str("exact"),
            Matcher::Subset => f.write_str("subset"),
            Matcher::Ignore => f.write_str("ignore"),
            Matcher::Set(p) => write!(f, "set:{}", p.as_written()),
            Matcher::Regex(p) => write!(f, "regex:{}", p.as_written()),
        }
    }
}

/// Failure to parse a `match` string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMatcherError(pub String);

impl fmt::Display for ParseMatcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown matcher `{}` — expected one of: exact, subset, set:<path>, regex:<path>, ignore",
            self.0
        )
    }
}

impl std::error::Error for ParseMatcherError {}

impl FromStr for Matcher {
    type Err = ParseMatcherError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "exact" => Ok(Matcher::Exact),
            "subset" => Ok(Matcher::Subset),
            "ignore" => Ok(Matcher::Ignore),
            _ => {
                if let Some(p) = s.strip_prefix("set:") {
                    Ok(Matcher::Set(Pointer::parse(p)))
                } else if let Some(p) = s.strip_prefix("regex:") {
                    Ok(Matcher::Regex(Pointer::parse(p)))
                } else {
                    Err(ParseMatcherError(s.to_string()))
                }
            }
        }
    }
}

impl Serialize for Matcher {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Matcher {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        s.parse().map_err(de::Error::custom)
    }
}
