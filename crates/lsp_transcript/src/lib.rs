//! `lsp_transcript` — the transcript model, its JSONL codec, and the matcher
//! engine that decides whether a live LSP session agrees with a recorded one.
//!
//! **This crate never talks to a server.** It is pure data plus comparison,
//! which is why ls00 can build and test it in full while `wolf lsp` does not
//! exist yet (D34: the server is the compiler, shipped by wolf-lang's s52 —
//! never by this repo). Spawning, framing, and replay live in `lsp_harness`.
//!
//! # The shape of the artifact
//!
//! ```text
//! {"transcript":1,"name":"fackr/open-hover","wolf_pin":"<sha>","profile":"fackr",
//!  "workspace":"vendor/upstream/samples","recorded":"2026-08-09"}
//! {"seq":1,"dir":"c2s","kind":"request","id":1,"method":"initialize","params":{…}}
//! {"seq":2,"dir":"s2c","kind":"response","id":1,"result":{…},"match":"subset"}
//! ```
//!
//! # Reading order
//!
//! - [`record`] — what a line is.
//! - [`matcher`] — how two lines are compared, and why the defaults are what
//!   they are.
//! - [`normalize`] — what is elided first, and the unconditional/opt-in split.
//! - [`defaults`] — the stability contract, in one table.
//! - [`encoding`] — byte offsets ⇄ LSP positions, written independently of
//!   the server's own conversion so the encoding suite compares two things.

pub mod defaults;
pub mod encoding;
pub mod jsonl;
pub mod matcher;
pub mod normalize;
pub mod pointer;
pub mod record;

pub use encoding::{Encoding, LineIndex, Position};
pub use jsonl::Error as JsonlError;
pub use matcher::{Matcher, Mismatch};
pub use normalize::{Normalizer, Stage};
pub use record::{Dir, Header, Kind, Record, Transcript};

/// Transcript format version, written as the header's `transcript` member.
///
/// Bumping it is its own commit with a migration note in `docs/`: every
/// recorded session in the repo is an input to that decision.
pub const FORMAT_VERSION: u32 = 1;
