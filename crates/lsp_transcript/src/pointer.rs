//! Payload-relative JSON pointers (RFC 6901 tokens, relaxed entry syntax).
//!
//! Matcher paths in a transcript are written against the *payload* — the
//! `result` of a response or the `params` of a notification — not against the
//! JSON-RPC envelope. `set:diagnostics` is the whole claim; making an author
//! write `set:/params/diagnostics` would couple every assertion to a wrapper
//! that carries no information.
//!
//! Entry syntax is relaxed in exactly one way: a leading `/` is optional, and
//! both `""` and `"/"` mean the payload root (which is what
//! `textDocument/documentSymbol` needs — its result *is* the array).
//! Within a path, RFC 6901 escaping applies: `~1` is `/`, `~0` is `~`.

use std::fmt;

use serde_json::Value;

/// A parsed, payload-relative pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pointer {
    tokens: Vec<String>,
    written: String,
}

impl Pointer {
    /// Parse relaxed pointer syntax. Never fails: any string is *some* path,
    /// and a path that does not resolve is reported at comparison time with
    /// the message the reader needs ("no such member"), not as a parse error
    /// divorced from the record it came from.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        let trimmed = s.strip_prefix('/').unwrap_or(s);
        let tokens = if trimmed.is_empty() {
            Vec::new()
        } else {
            trimmed.split('/').map(unescape).collect()
        };
        Self {
            tokens,
            written: s.to_string(),
        }
    }

    /// The path exactly as it appears in the transcript, so a round-trip does
    /// not rewrite `set:diagnostics` into `set:/diagnostics`.
    #[must_use]
    pub fn as_written(&self) -> &str {
        &self.written
    }

    /// True for the payload root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.tokens.is_empty()
    }

    #[must_use]
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }
}

impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.tokens.is_empty() {
            return Ok(());
        }
        for t in &self.tokens {
            write!(f, "/{}", escape(t))?;
        }
        Ok(())
    }
}

/// Borrow the value at `ptr`, if it is there.
#[must_use]
pub fn resolve<'a>(value: &'a Value, ptr: &Pointer) -> Option<&'a Value> {
    let mut cur = value;
    for token in &ptr.tokens {
        cur = step(cur, token)?;
    }
    Some(cur)
}

/// Remove and return the value at `ptr`, leaving the parent otherwise intact.
///
/// Removal rather than borrow-and-skip is what lets `set:`/`regex:` compare
/// "everything except this member" without a second traversal that has to
/// remember which member to duck.
pub fn take(value: &mut Value, ptr: &Pointer) -> Option<Value> {
    let Some((last, parents)) = ptr.tokens.split_last() else {
        // Root: swap the whole payload out.
        return Some(std::mem::replace(value, Value::Null));
    };
    let mut cur = value;
    for token in parents {
        cur = step_mut(cur, token)?;
    }
    match cur {
        Value::Object(map) => map.remove(last),
        Value::Array(items) => {
            let idx: usize = last.parse().ok()?;
            (idx < items.len()).then(|| items.remove(idx))
        }
        _ => None,
    }
}

fn step<'a>(value: &'a Value, token: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => map.get(token),
        Value::Array(items) => items.get(token.parse::<usize>().ok()?),
        _ => None,
    }
}

fn step_mut<'a>(value: &'a mut Value, token: &str) -> Option<&'a mut Value> {
    match value {
        Value::Object(map) => map.get_mut(token),
        Value::Array(items) => {
            let idx: usize = token.parse().ok()?;
            items.get_mut(idx)
        }
        _ => None,
    }
}

/// RFC 6901 escaping, for rendering a token back into a path.
#[must_use]
pub fn escape(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn unescape(token: &str) -> String {
    // Order matters: `~1` must be decoded before `~0`, or `~01` round-trips
    // to `/` instead of `~1`.
    token.replace("~1", "/").replace("~0", "~")
}
