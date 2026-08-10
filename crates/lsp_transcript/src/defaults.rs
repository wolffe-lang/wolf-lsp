//! The default matcher per method — **the stability contract**.
//!
//! One table, in one file, so that "what does this suite actually promise?"
//! has a single answer a reviewer can read in a minute. Every entry is
//! justified by what the protocol pins, not by what happens to be convenient:
//!
//! - The protocol says a server *may* advertise more than a client asked
//!   about, so `ServerCapabilities` is [`Matcher::Subset`]. A server that
//!   gains a capability must not turn the suite red; one that loses a
//!   capability a transcript relied on must. This is the single most
//!   important default in the file.
//! - The protocol does not specify the order of diagnostics, symbols, or
//!   locations, so those are multisets. Their *contents* — ranges, codes,
//!   severities — are behavior, and stay exact inside the multiset.
//! - Formatting output is a byte-for-byte claim (`wolf fmt` is the one
//!   canonical style, D34), so it is [`Matcher::Exact`].
//! - Incidental server chatter (`window/logMessage`, `$/progress`) carries no
//!   claim and is ignored; a transcript that wants to assert on it says so.
//!
//! # What this table does *not* cover
//!
//! It is keyed by method, and a method has two answers: a result and an error.
//! Every entry below describes the **result** shape, so an error response —
//! whose payload is `{code, message}` whatever the method — must not be routed
//! through it. That case is handled one level up, in
//! [`crate::record::Record::effective_matcher`], which sends error responses to
//! [`Matcher::Subset`] before consulting this file. Putting the rule here
//! instead would mean duplicating it into every arm.

use crate::matcher::Matcher;
use crate::pointer::Pointer;
use crate::record::Kind;

/// Payload-root multiset: the result *is* the array.
fn set_root() -> Matcher {
    Matcher::Set(Pointer::parse(""))
}

fn set_at(path: &str) -> Matcher {
    Matcher::Set(Pointer::parse(path))
}

/// The matcher a record gets when it does not declare one.
///
/// `method` is the method of the *exchange*: for a response record it is the
/// method of the request that shares its id, which [`crate::record::Transcript`]
/// correlates — a response line in the file carries no method of its own.
#[must_use]
pub fn for_method(method: Option<&str>, kind: Kind) -> Matcher {
    match kind {
        Kind::Response => match method {
            // Forward compatibility, by design.
            Some("initialize") => Matcher::Subset,
            // `null` and nothing else.
            Some("shutdown") => Matcher::Exact,
            // Canonical formatter output — the whole claim is the bytes.
            Some(
                "textDocument/formatting"
                | "textDocument/rangeFormatting"
                | "textDocument/onTypeFormatting",
            ) => Matcher::Exact,
            // Arrays LSP leaves unordered.
            Some(
                "textDocument/documentSymbol"
                | "workspace/symbol"
                | "textDocument/definition"
                | "textDocument/declaration"
                | "textDocument/typeDefinition"
                | "textDocument/implementation"
                | "textDocument/references"
                | "textDocument/codeAction",
            ) => set_root(),
            Some("textDocument/completion") => set_at("items"),
            // Hover contents are prose D22 owns upstream; the server may add
            // `range` without changing what it means.
            Some("textDocument/hover") => Matcher::Subset,
            _ => Matcher::Subset,
        },
        Kind::Notification => match method {
            Some("textDocument/publishDiagnostics") => set_at("diagnostics"),
            Some(
                "window/logMessage" | "window/showMessage" | "telemetry/event" | "$/progress"
                | "$/logTrace",
            ) => Matcher::Ignore,
            _ => Matcher::Subset,
        },
        // Server-to-client requests. The shim treats these as best-effort
        // (report 09 §transport), so the suite asserts shape, not identity.
        Kind::Request => match method {
            Some("window/workDoneProgress/create") => Matcher::Ignore,
            _ => Matcher::Subset,
        },
    }
}
