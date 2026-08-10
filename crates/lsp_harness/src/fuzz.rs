//! Fuzzed partial-edit sessions — random splices through `didChange`, and
//! three oracles of increasing strength.
//!
//! Sprint §7. The generator biases toward the two places overlay bookkeeping
//! actually breaks: **token boundaries** (where a parser's recovery decides
//! what to resynchronize on) and **mid-multibyte-character offsets** (where a
//! byte-indexed store and a code-unit-indexed client disagree). A uniform
//! random splice over a file spends most of its budget inside comments.
//!
//! # Seeds
//!
//! Explicit, always, and never derived from a path (ls00 §4). A seed that
//! depends on where the repo is checked out makes a failure unreproducible on
//! the machine that has to fix it, and makes CI's "same seed every run" quietly
//! different per runner.
//!
//! # The oracles
//!
//! 1. **Liveness** — no panic, no protocol violation, no unanswered request.
//!    The floor: the server must still be there and still be answering.
//! 2. **Round-trip** — after N splices, apply the inverse sequence in reverse
//!    and assert the diagnostics set is *identical* to the initial one. Cheap
//!    and brutal: any overlay state that survives an edit and its undo is a
//!    leak, and the assertion needs no oracle for what the diagnostics *should*
//!    be, only that returning to the same bytes returns to the same answer.
//! 3. **Sync-mode equivalence** — the same final text delivered as one whole
//!    replacement and as a keystroke storm of whole replacements must produce
//!    identical diagnostics, `documentSymbol`, and formatting. This is the one
//!    the tier-0 clients differ on (report 09: fackr sends full text every
//!    keystroke; facsimile debounces 500 ms and pins `version` to 1), which is
//!    why it is a harness target rather than a client one.

use std::fmt;
use std::path::Path;
use std::time::Instant;

use serde_json::{Value, json};

use crate::profiles::Profile;
use crate::session::{self, Session, file_uri};

/// SplitMix64 — a deterministic PRNG in twelve lines.
///
/// Not a dependency, because a fuzzer whose reproducibility depends on a
/// crate's version is a fuzzer whose old seeds stop meaning anything after a
/// `cargo update`. The sequence this produces is fixed forever by the
/// constants below, so a seed in a bug report reproduces in five years.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, n)`. `n == 0` yields 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// One edit, and enough to undo it.
///
/// **Bytes, not `String`.** The generator lands mid-character on purpose, so a
/// splice's removed text is routinely not valid UTF-8; storing it as a `String`
/// would run it through a lossy conversion and the "inverse" would reinsert
/// U+FFFD instead of the original bytes. The round-trip oracle would then fail
/// on the harness's own arithmetic and blame the server — a false positive that
/// costs an afternoon each time it happens.
///
/// What the *server* sees is still valid UTF-8: `didChange` text is a JSON
/// string, so [`change`] lossily converts on the way out, exactly as a real
/// editor would have to. The harness keeps the exact bytes; the wire carries
/// what the wire can carry; and because the round trip restores the exact
/// bytes, both endpoints agree at the two moments the oracle compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Splice {
    pub lo: usize,
    pub hi: usize,
    pub inserted: Vec<u8>,
    /// What `[lo, hi)` held before, so the inverse is constructible without
    /// keeping every intermediate document.
    pub removed: Vec<u8>,
}

impl Splice {
    /// The edit that undoes this one, given it is applied to the text this one
    /// produced.
    #[must_use]
    pub fn inverse(&self) -> Splice {
        Splice {
            lo: self.lo,
            hi: self.lo + self.inserted.len(),
            inserted: self.removed.clone(),
            removed: self.inserted.clone(),
        }
    }

    /// Apply to a buffer.
    #[must_use]
    pub fn apply(&self, src: &[u8]) -> Vec<u8> {
        crate::drive::splice(src, self.lo, self.hi, &self.inserted)
    }
}

/// Fragments the generator splices in. Chosen to be things a parser has an
/// opinion about, not lorem ipsum: an unbalanced brace, a keyword, an operator
/// that changes precedence, and text above the BMP.
const FRAGMENTS: &[&str] = &[
    "", "{", "}", "(", ")", "\"", "fn ", "let ", "var ", " -> ", "!", "?", ",", ";", "\n", "\t",
    "0", "x", "🐺", "é", "中", "\u{200D}", "//", "\"\"\"", "{x}",
];

/// Generate one splice over `src`.
///
/// Offsets land on code-point boundaries **most** of the time and deliberately
/// inside a multi-byte sequence the rest: a client cannot produce a mid-code-
/// point offset through positions, but a buggy conversion can, and the server
/// must survive being handed one rather than panicking on a slice.
#[must_use]
pub fn generate(rng: &mut Rng, src: &[u8]) -> Splice {
    let len = src.len();
    let mut lo = rng.below(len + 1);
    // Bias to token boundaries: snap to a nearby non-alphanumeric byte, which
    // is where recovery decisions get made.
    if rng.below(100) < 55 {
        let window = 24;
        let from = lo.saturating_sub(window);
        let to = (lo + window).min(len);
        if let Some(i) = (from..to).find(|&i| !src[i].is_ascii_alphanumeric()) {
            lo = i;
        }
    }
    // Seven splices in eight get snapped back to a code-point boundary; the
    // eighth is left mid-character on purpose. `lo > 0` guards the walk: an
    // already-spliced buffer can begin with a continuation byte, and a
    // saturating decrement would spin rather than underflow.
    if rng.below(8) != 0 {
        while lo > 0 && lo < len && is_continuation(src[lo]) {
            lo -= 1;
        }
    }
    let max_delete = (len - lo).min(48);
    let mut hi = lo + rng.below(max_delete + 1);
    if rng.below(8) != 0 {
        while hi < len && is_continuation(src[hi]) {
            hi += 1;
        }
    }
    let mut inserted = Vec::new();
    for _ in 0..rng.below(4) {
        inserted.extend_from_slice(rng.pick(FRAGMENTS).as_bytes());
    }
    Splice {
        lo,
        hi,
        inserted,
        removed: src[lo..hi].to_vec(),
    }
}

fn is_continuation(b: u8) -> bool {
    (0x80..0xC0).contains(&b)
}

/// What a fuzz session concluded.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub seed: u64,
    pub sample: String,
    pub splices: usize,
    /// Oracle failures, each already phrased as the bug it is.
    pub failures: Vec<String>,
    /// The splice sequence, so a failure is reproducible without the seed
    /// (which reproduces it only against the same generator).
    pub history: Vec<Splice>,
}

impl Outcome {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Errors that stop a fuzz run — distinct from oracle failures, which *are*
/// the result.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Session(session::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Session(e) => write!(f, "{e}"),
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

/// The observable server state for a document: what all three oracles compare.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub diagnostics: Vec<Value>,
    pub symbols: Value,
    pub formatting: Value,
}

/// Run one seeded fuzz session.
pub fn run(
    bin: &Path,
    workspace: &Path,
    sample: &str,
    profile: &Profile,
    seed: u64,
    splices: usize,
) -> Result<Outcome, Error> {
    let path = workspace.join(sample);
    let original = std::fs::read(&path)?;
    let uri = file_uri(&path);
    let mut rng = Rng::new(seed);
    let mut failures = Vec::new();
    let mut history = Vec::new();

    let mut session = Session::spawn(bin, workspace)?;
    session.initialize(&profile.capabilities)?;
    let mut version = 1i64;
    open(&mut session, &uri, &original, version)?;
    let initial = read_state(&mut session, &uri)?;

    // --- oracles 1 and 2 ------------------------------------------------
    let mut text = original.clone();
    for _ in 0..splices {
        let splice = generate(&mut rng, &text);
        text = splice.apply(&text);
        history.push(splice);
        version += 1;
        change(&mut session, &uri, &text, version)?;
        // Oracle 1, continuously: every request must still be answered, and
        // interleaving them with the edits is the point — a server that only
        // survives a quiet document survives nothing.
        if let Err(e) = probe(&mut session, &uri, &mut rng) {
            failures.push(format!(
                "liveness: the server stopped answering after {} splice(s): {e}",
                history.len()
            ));
            return Ok(Outcome {
                seed,
                sample: sample.to_string(),
                splices,
                failures,
                history,
            });
        }
    }

    // Undo, in reverse. The bytes must come back, and so must the answers.
    for splice in history.iter().rev() {
        text = splice.inverse().apply(&text);
        version += 1;
        change(&mut session, &uri, &text, version)?;
    }
    if text != original {
        failures.push(
            "round-trip: the harness's own inverse sequence did not restore the bytes \
             — this is a harness bug, not a server one, and the oracle below would have \
             been meaningless"
                .to_string(),
        );
    } else {
        let restored = read_state(&mut session, &uri)?;
        if restored.diagnostics != initial.diagnostics {
            failures.push(format!(
                "round-trip: after {splices} splices and their exact inverses the document \
                 holds the original bytes, but the diagnostics differ — {} before, {} after. \
                 Overlay state survived an edit and its undo.",
                initial.diagnostics.len(),
                restored.diagnostics.len()
            ));
        }
        if restored.symbols != initial.symbols {
            failures.push(
                "round-trip: documentSymbol differs after an edit and its exact inverse"
                    .to_string(),
            );
        }
    }

    // --- oracle 3: sync-mode equivalence --------------------------------
    let storm_target = {
        let mut rng2 = Rng::new(seed ^ 0x5DEE_CE66_D2B0_79F5);
        let mut t = original.clone();
        let mut steps = Vec::new();
        for _ in 0..splices.min(12) {
            let s = generate(&mut rng2, &t);
            t = s.apply(&t);
            steps.push(t.clone());
        }
        (t, steps)
    };
    let (final_text, steps) = storm_target;

    // (a) one whole-document replacement.
    version += 1;
    change(&mut session, &uri, &final_text, version)?;
    let at_once = read_state(&mut session, &uri)?;

    // (b) the same destination reached one keystroke-shaped full replacement
    //     at a time — fackr's actual behavior.
    version += 1;
    change(&mut session, &uri, &original, version)?;
    for step in &steps {
        version += 1;
        change(&mut session, &uri, step, version)?;
    }
    let by_storm = read_state(&mut session, &uri)?;

    if at_once.diagnostics != by_storm.diagnostics {
        failures.push(format!(
            "sync-mode equivalence: the same final text produced {} diagnostic(s) when \
             delivered as one replacement and {} when delivered as {} — the path taken to a \
             document changed the answer",
            at_once.diagnostics.len(),
            by_storm.diagnostics.len(),
            steps.len()
        ));
    }
    if at_once.symbols != by_storm.symbols {
        failures.push(
            "sync-mode equivalence: documentSymbol differs between one replacement and \
             a keystroke storm reaching the same text"
                .to_string(),
        );
    }
    if at_once.formatting != by_storm.formatting {
        failures.push(
            "sync-mode equivalence: textDocument/formatting differs between one replacement \
             and a keystroke storm reaching the same text"
                .to_string(),
        );
    }

    // A range-carrying `contentChange` against a server that negotiated FULL
    // sync: the declared sync mode must be the one honored. Corruption here is
    // the classic "server accepts deltas it never advertised" bug.
    let before = read_state(&mut session, &uri)?;
    version += 1;
    session.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": uri, "version": version},
               "contentChanges": [{"range": {"start": {"line": 0, "character": 0},
                                             "end": {"line": 0, "character": 0}},
                                   "rangeLength": 0, "text": "XXX"}]}),
    )?;
    let after = read_state(&mut session, &uri)?;
    if after.diagnostics != before.diagnostics {
        failures.push(
            "sync-mode: the server advertised textDocumentSync FULL but changed state on a \
             range-carrying contentChange — a client sending deltas it was never told to \
             send would silently corrupt the buffer"
                .to_string(),
        );
    }

    session.shutdown_exit()?;
    Ok(Outcome {
        seed,
        sample: sample.to_string(),
        splices,
        failures,
        history,
    })
}

/// `didOpen`, and wait for the first publish. Public so the oracle tests can
/// plant a bug and watch the same comparison catch it — an oracle demonstrated
/// only against a server that passes is an oracle nobody has seen work.
pub fn open(
    session: &mut Session,
    uri: &str,
    text: &[u8],
    version: i64,
) -> Result<(), session::Error> {
    let started = Instant::now();
    session.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": uri, "languageId": "wolf", "version": version,
                                "text": String::from_utf8_lossy(text)}}),
    )?;
    let _ = session.notification("textDocument/publishDiagnostics", Some(uri), started)?;
    Ok(())
}

/// `didChange` with a whole new text (full sync is what v0 negotiates).
///
/// The text is lossily converted: `didChange` carries a JSON string, so
/// invalid UTF-8 cannot reach a real server either. See [`Splice`].
pub fn change(
    session: &mut Session,
    uri: &str,
    text: &[u8],
    version: i64,
) -> Result<(), session::Error> {
    session.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": uri, "version": version},
               "contentChanges": [{"text": String::from_utf8_lossy(text)}]}),
    )
}

/// One request of each kind, so a hang or a dropped response shows up.
fn probe(session: &mut Session, uri: &str, rng: &mut Rng) -> Result<(), session::Error> {
    let line = rng.below(40) as u64;
    let character = rng.below(80) as u64;
    let methods: [(&str, Value); 2] = [
        (
            "textDocument/hover",
            json!({"textDocument": {"uri": uri},
                   "position": {"line": line, "character": character}}),
        ),
        (
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": uri}}),
        ),
    ];
    for (method, params) in methods {
        let started = Instant::now();
        let id = session.request(method, params)?;
        let (resp, _) = session.response(id, started)?;
        // `ContentModified` is a legal answer under a write storm and is not a
        // failure; a *missing* answer is, and `response` already enforces that.
        let _ = resp;
    }
    Ok(())
}

/// Save-then-read: force a publish and collect everything comparable.
///
/// `didSave` flushes the debounce immediately, which is what makes this
/// deterministic — waiting out the 100 ms window would race the next edit.
pub fn read_state(session: &mut Session, uri: &str) -> Result<State, session::Error> {
    let started = Instant::now();
    session.notify(
        "textDocument/didSave",
        json!({"textDocument": {"uri": uri}}),
    )?;
    let (published, _) =
        session.notification("textDocument/publishDiagnostics", Some(uri), started)?;
    let diagnostics = published
        .pointer("/params/diagnostics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let started = Instant::now();
    let id = session.request(
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": uri}}),
    )?;
    let symbols = session.response(id, started)?.0;

    let started = Instant::now();
    let id = session.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": uri},
               "options": {"tabSize": 4, "insertSpaces": true}}),
    )?;
    let formatting = session.response(id, started)?.0;

    Ok(State {
        diagnostics,
        symbols: symbols.get("result").cloned().unwrap_or(Value::Null),
        formatting: formatting.get("result").cloned().unwrap_or(Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = "fn main() -> !int {\n    let s = \"héllo 🐺\"\n    0\n}\n".as_bytes();

    #[test]
    fn the_same_seed_generates_the_same_sequence() {
        let a: Vec<Splice> = {
            let mut r = Rng::new(7);
            let mut t = SRC.to_vec();
            (0..20)
                .map(|_| {
                    let s = generate(&mut r, &t);
                    t = s.apply(&t);
                    s
                })
                .collect()
        };
        let b: Vec<Splice> = {
            let mut r = Rng::new(7);
            let mut t = SRC.to_vec();
            (0..20)
                .map(|_| {
                    let s = generate(&mut r, &t);
                    t = s.apply(&t);
                    s
                })
                .collect()
        };
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_generate_different_sequences() {
        let one = |seed| {
            let mut r = Rng::new(seed);
            generate(&mut r, SRC)
        };
        assert_ne!(one(1), one(2));
    }

    #[test]
    fn every_splice_and_its_inverse_restore_the_bytes() {
        // The property oracle 2 rests on. If this were false the oracle would
        // pass vacuously, which is the worst way for a test to be wrong.
        let mut rng = Rng::new(0xC0FFEE);
        let mut text = SRC.to_vec();
        let mut history = Vec::new();
        for _ in 0..200 {
            let s = generate(&mut rng, &text);
            text = s.apply(&text);
            history.push(s);
        }
        for s in history.iter().rev() {
            text = s.inverse().apply(&text);
        }
        assert_eq!(text, SRC);
    }

    #[test]
    fn splices_are_generated_across_a_shrinking_and_growing_document() {
        // Includes the degenerate case: a document the generator emptied.
        let mut rng = Rng::new(99);
        let mut text: Vec<u8> = Vec::new();
        for _ in 0..500 {
            let s = generate(&mut rng, &text);
            assert!(s.lo <= text.len(), "lo {} > len {}", s.lo, text.len());
            assert!(s.hi <= text.len(), "hi {} > len {}", s.hi, text.len());
            text = s.apply(&text);
        }
    }

    #[test]
    fn some_splices_land_mid_multibyte_which_is_the_whole_point() {
        let mut rng = Rng::new(4242);
        let src = "🐺🐺🐺🐺🐺🐺🐺🐺🐺🐺🐺🐺🐺🐺🐺🐺".as_bytes();
        let mid = (0..400)
            .filter(|_| {
                let s = generate(&mut rng, src);
                !s.lo.is_multiple_of(4) || !s.hi.is_multiple_of(4)
            })
            .count();
        assert!(
            mid > 0,
            "the generator never produced a mid-character offset"
        );
    }

    #[test]
    fn generated_text_is_never_sliced_out_of_bounds() {
        let mut rng = Rng::new(1);
        for _ in 0..1000 {
            let s = generate(&mut rng, SRC);
            let _ = s.apply(SRC);
        }
    }
}
