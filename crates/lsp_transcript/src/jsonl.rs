//! JSONL codec — parse and canonically serialize a transcript.
//!
//! Canonical form is load-bearing (sprint §2): **sorted keys, LF endings, a
//! trailing newline, one message per line**. Re-recording a session must
//! produce a diff a human can review, not a reshuffle that hides the one line
//! that actually changed.

use std::fmt;

use serde_json::Value;

use crate::record::{Header, Record, Transcript, sort_keys};

/// A parse failure, located by line so the message points at the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// 1-based line number in the JSONL file.
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for Error {}

/// Parse a transcript from JSONL text.
///
/// Blank lines are skipped; `#`-prefixed lines are not a thing, deliberately —
/// a transcript is data a tool writes, and a comment syntax invites hand
/// edits that the re-record ritual would silently discard.
pub fn parse(text: &str) -> Result<Transcript, Error> {
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l))
        .filter(|(_, l)| !l.trim().is_empty());

    let (hline, htext) = lines.next().ok_or(Error {
        line: 1,
        message: "empty transcript: the first line must be the header record".to_string(),
    })?;
    let header: Header = serde_json::from_str(htext).map_err(|e| Error {
        line: hline,
        message: format!("header: {e}"),
    })?;

    let mut records = Vec::new();
    for (n, text) in lines {
        let rec: Record = serde_json::from_str(text).map_err(|e| Error {
            line: n,
            message: e.to_string(),
        })?;
        records.push(rec);
    }

    Ok(Transcript { header, records })
}

/// Serialize a transcript to canonical JSONL.
///
/// # Panics
///
/// Only if a `Record` fails to serialize into a `serde_json::Value`, which
/// cannot happen for this type — every field is already `Value`-representable.
#[must_use]
pub fn to_string(transcript: &Transcript) -> String {
    let mut out = String::new();
    push_line(&mut out, &transcript.header);
    for rec in &transcript.records {
        push_line(&mut out, rec);
    }
    out
}

fn push_line<T: serde::Serialize>(out: &mut String, value: &T) {
    let mut v: Value =
        serde_json::to_value(value).expect("transcript records are Value-representable");
    sort_keys(&mut v);
    out.push_str(&serde_json::to_string(&v).expect("a serde_json::Value always serializes"));
    out.push('\n');
}
