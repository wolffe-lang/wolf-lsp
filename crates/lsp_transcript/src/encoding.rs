//! `positionEncoding` — byte offsets ⇄ LSP positions, in all three kinds.
//!
//! **This is a second, independent implementation on purpose.** The server has
//! one (`wolf_lsp::positions`, confined to a single module by L2); this repo
//! may not link against it (D34), and would learn nothing by doing so — a
//! conformance harness that computes positions with the code under test
//! asserts only that the code equals itself. Everything here is written from
//! the 3.17 specification text, and the encoding suite (ls01 §5) is the
//! comparison of the two.
//!
//! # The rules this file implements
//!
//! - A `character` counts **code units of the negotiated encoding** from the
//!   start of the line: bytes for `utf-8`, UTF-16 code units for `utf-16`
//!   (astral code points cost two), code points for `utf-32`.
//! - `character` past the end of the line **clamps to the line end**; `line`
//!   past the end of the document clamps to the last line.
//! - A `character` that would land **inside** a code point resolves to that
//!   code point's start. The protocol calls this out for surrogate halves and
//!   it is the only defined answer; the astral fixtures assert it is *stable*,
//!   not merely defined.
//!
//! # Lines are split on `\n` and nothing else
//!
//! Not a shortcut — a compatibility requirement. The server derives positions
//! from `wolf_span::LineIndex`, which splits on `\n` alone, so a lone `\r`
//! is ordinary line content and a CRLF document carries a `\r` as the last
//! unit of every line. Splitting on `\r\n` here would put this file one column
//! away from the server on every CRLF buffer and the one-truth check would
//! report a divergence that exists only in the harness. ls01 §5's CRLF case
//! pins the agreement.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A negotiated position encoding.
/// The names are spelled out rather than derived from a `rename_all` rule:
/// `PositionEncodingKind` is `utf-8`, and every casing convention serde
/// offers produces `utf8` or `utf_8` instead. A profile document and the wire
/// must use the same string or the two halves of the negotiation assertion
/// stop being about the same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Encoding {
    /// `utf-8` — `character` is a byte column. No conversion at all.
    #[serde(rename = "utf-8")]
    Utf8,
    /// `utf-16` — the protocol's mandatory default and its oldest wart.
    #[serde(rename = "utf-16")]
    Utf16,
    /// `utf-32` — code points; what rope-backed editors natively count.
    #[serde(rename = "utf-32")]
    Utf32,
}

impl Encoding {
    /// The wire string (`PositionEncodingKind`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Encoding::Utf8 => "utf-8",
            Encoding::Utf16 => "utf-16",
            Encoding::Utf32 => "utf-32",
        }
    }

    /// Every encoding, in the order the suite iterates them.
    #[must_use]
    pub fn all() -> [Encoding; 3] {
        [Encoding::Utf8, Encoding::Utf16, Encoding::Utf32]
    }
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Failure to parse a `PositionEncodingKind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEncodingError(pub String);

impl fmt::Display for ParseEncodingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown positionEncoding `{}` — expected utf-8, utf-16, or utf-32",
            self.0
        )
    }
}

impl std::error::Error for ParseEncodingError {}

impl FromStr for Encoding {
    type Err = ParseEncodingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "utf-8" => Encoding::Utf8,
            "utf-16" => Encoding::Utf16,
            "utf-32" => Encoding::Utf32,
            other => return Err(ParseEncodingError(other.to_string())),
        })
    }
}

/// A zero-based LSP position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    #[must_use]
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.character)
    }
}

/// Byte offsets of the first byte of each line. Always non-empty.
///
/// Mirrors `wolf_span::LineIndex` deliberately — see the module note on why
/// `\n` is the only terminator.
#[derive(Debug, Clone)]
pub struct LineIndex {
    starts: Vec<u32>,
    len: u32,
}

impl LineIndex {
    /// Index a buffer.
    ///
    /// # Panics
    ///
    /// If the buffer exceeds `u32::MAX` bytes, which the compiler's own span
    /// representation also refuses.
    #[must_use]
    pub fn new(src: &[u8]) -> Self {
        let len = u32::try_from(src.len()).expect("source exceeds u32::MAX bytes");
        let mut starts = vec![0u32];
        for (i, &b) in src.iter().enumerate() {
            if b == b'\n' {
                starts.push(i as u32 + 1);
            }
        }
        Self { starts, len }
    }

    /// Number of lines. A trailing newline opens a final empty line.
    #[must_use]
    pub fn line_count(&self) -> u32 {
        self.starts.len() as u32
    }

    /// Byte range `[start, end)` of `line`, excluding the terminating `\n`.
    /// A line index past the end clamps to the last line.
    #[must_use]
    pub fn line_range(&self, line: u32) -> (u32, u32) {
        let line = line.min(self.line_count() - 1) as usize;
        let start = self.starts[line];
        let end = self
            .starts
            .get(line + 1)
            .map_or(self.len, |&next| next.saturating_sub(1));
        (start, end)
    }

    /// The line containing `offset`, clamping past the end.
    #[must_use]
    pub fn line_of(&self, offset: u32) -> u32 {
        match self.starts.binary_search(&offset.min(self.len)) {
            Ok(line) => line as u32,
            Err(insert) => (insert - 1) as u32,
        }
    }
}

/// Length of the UTF-8 sequence starting with lead byte `b`.
///
/// Invalid lead bytes count as one byte so that conversion is **total**: a
/// harness that panics on malformed input cannot test a server's handling of
/// malformed input, and the fuzzer (ls01 §7) splices mid-character on purpose.
#[must_use]
pub fn seq_len(b: u8) -> usize {
    match b {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

/// Code units one UTF-8 sequence of `len` bytes occupies in `enc`.
///
/// The whole UTF-16 wart in four lines: a four-byte sequence is one code
/// point, one grapheme, four bytes, and **two** UTF-16 units.
#[must_use]
pub fn units(enc: Encoding, len: usize) -> u32 {
    match enc {
        Encoding::Utf8 => len as u32,
        Encoding::Utf32 => 1,
        Encoding::Utf16 => {
            if len == 4 {
                2
            } else {
                1
            }
        }
    }
}

/// Code units in a whole line slice, under `enc`.
#[must_use]
pub fn line_units(line: &[u8], enc: Encoding) -> u32 {
    if enc == Encoding::Utf8 {
        return line.len() as u32;
    }
    let mut n = 0;
    let mut i = 0;
    while i < line.len() {
        let len = seq_len(line[i]).min(line.len() - i);
        n += units(enc, len);
        i += len;
    }
    n
}

/// Byte offset → LSP position. Offsets past EOF clamp to the end.
#[must_use]
pub fn offset_to_position(src: &[u8], index: &LineIndex, offset: u32, enc: Encoding) -> Position {
    let offset = offset.min(src.len() as u32);
    let line = index.line_of(offset);
    let (start, _) = index.line_range(line);
    let prefix = &src[start as usize..offset as usize];
    Position {
        line,
        character: line_units(prefix, enc),
    }
}

/// LSP position → byte offset, with the specification's clamping rules.
#[must_use]
pub fn position_to_offset(src: &[u8], index: &LineIndex, pos: Position, enc: Encoding) -> u32 {
    let line = pos.line.min(index.line_count() - 1);
    let (start, end) = index.line_range(line);
    let bytes = &src[start as usize..end as usize];
    if enc == Encoding::Utf8 {
        return start + pos.character.min(bytes.len() as u32);
    }
    let mut i = 0usize;
    let mut character = 0u32;
    while i < bytes.len() && character < pos.character {
        let len = seq_len(bytes[i]).min(bytes.len() - i);
        let step = units(enc, len);
        if character + step > pos.character {
            // Landing inside a code point resolves to its start. For utf-16
            // that is the low half of a surrogate pair; the fixture asserts
            // this answer is stable, not just defined.
            break;
        }
        character += step;
        i += len;
    }
    start + i as u32
}

/// A byte span `[lo, hi)` → an LSP range, as a `(start, end)` pair.
#[must_use]
pub fn span_to_range(
    src: &[u8],
    index: &LineIndex,
    lo: u32,
    hi: u32,
    enc: Encoding,
) -> (Position, Position) {
    (
        offset_to_position(src, index, lo, enc),
        offset_to_position(src, index, hi, enc),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // The specification's own worked example: in `a𐐀b`, `b` sits at utf-16
    // character 3, utf-8 character 5, utf-32 character 2.
    #[test]
    fn the_spec_surrogate_example() {
        let src = "a\u{10400}b".as_bytes();
        let idx = LineIndex::new(src);
        for (enc, ch) in [
            (Encoding::Utf8, 5),
            (Encoding::Utf16, 3),
            (Encoding::Utf32, 2),
        ] {
            assert_eq!(
                offset_to_position(src, &idx, 5, enc),
                Position::new(0, ch),
                "{enc}"
            );
            assert_eq!(
                position_to_offset(src, &idx, Position::new(0, ch), enc),
                5,
                "{enc}"
            );
        }
    }

    #[test]
    fn a_character_inside_a_surrogate_pair_resolves_to_its_start() {
        let src = "\u{1F43A}x".as_bytes(); // 🐺
        let idx = LineIndex::new(src);
        // character 1 is the *low half* of the pair — not addressable.
        assert_eq!(
            position_to_offset(src, &idx, Position::new(0, 1), Encoding::Utf16),
            0
        );
        // character 2 is `x`.
        assert_eq!(
            position_to_offset(src, &idx, Position::new(0, 2), Encoding::Utf16),
            4
        );
    }

    #[test]
    fn clamping_is_defined_in_both_dimensions() {
        let src = b"ab\ncd";
        let idx = LineIndex::new(src);
        assert_eq!(
            position_to_offset(src, &idx, Position::new(0, 99), Encoding::Utf16),
            2,
            "character past the line end clamps to the line end"
        );
        assert_eq!(
            position_to_offset(src, &idx, Position::new(99, 0), Encoding::Utf16),
            3,
            "line past EOF clamps to the last line"
        );
    }

    #[test]
    fn a_lone_carriage_return_is_line_content_not_a_terminator() {
        // The server's LineIndex splits on `\n` alone; if this file split on
        // `\r\n` the two would disagree by one column on every CRLF buffer.
        let src = b"ab\r\ncd";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_count(), 2);
        // The `\r` is the third unit of line 0.
        assert_eq!(
            offset_to_position(src, &idx, 3, Encoding::Utf16),
            Position::new(0, 3)
        );
        assert_eq!(
            offset_to_position(src, &idx, 4, Encoding::Utf16),
            Position::new(1, 0)
        );
    }

    #[test]
    fn byte_length_utf16_length_and_char_count_are_four_different_numbers() {
        // `é` (2 bytes) `中` (3) `🐺` (4) plus a combining mark and a ZWJ
        // family: the four counts LSP, the editor, and the user each care
        // about, and the grapheme count nothing in LSP uses.
        let line = "é中🐺e\u{0301}👨\u{200D}👩\u{200D}👧".as_bytes();
        assert_eq!(line.len(), 2 + 3 + 4 + 1 + 2 + 4 + 3 + 4 + 3 + 4);
        assert_eq!(line_units(line, Encoding::Utf8), line.len() as u32);
        // code points: é 中 🐺 e ́ 👨 ZWJ 👩 ZWJ 👧 = 10
        assert_eq!(line_units(line, Encoding::Utf32), 10);
        // utf-16: the four astral code points cost two units each.
        assert_eq!(line_units(line, Encoding::Utf16), 10 + 4);
    }

    #[test]
    fn round_trips_at_every_code_point_boundary_in_every_encoding() {
        let src = "let s = \"héllo 🐺\"\nlet 中 = 1\n\ttab\r\n".as_bytes();
        let idx = LineIndex::new(src);
        for enc in Encoding::all() {
            let mut off = 0usize;
            while off < src.len() {
                let pos = offset_to_position(src, &idx, off as u32, enc);
                assert_eq!(
                    position_to_offset(src, &idx, pos, enc),
                    off as u32,
                    "{enc} at byte {off}"
                );
                off += seq_len(src[off]);
            }
        }
    }

    #[test]
    fn conversion_is_total_on_invalid_utf8() {
        // The fuzzer splices mid-character on purpose; a panic here would be
        // the harness failing, not the server.
        let src = &[0xF0, 0x9F, b'x', b'\n', 0xC3][..];
        let idx = LineIndex::new(src);
        for enc in Encoding::all() {
            for off in 0..=src.len() as u32 {
                let pos = offset_to_position(src, &idx, off, enc);
                let _ = position_to_offset(src, &idx, pos, enc);
            }
        }
    }

    #[test]
    fn encoding_names_round_trip_through_the_wire_form() {
        for enc in Encoding::all() {
            assert_eq!(enc.as_str().parse::<Encoding>().unwrap(), enc);
        }
        assert!("utf-7".parse::<Encoding>().is_err());
    }

    #[test]
    fn serde_uses_the_protocol_spelling_not_a_casing_convention() {
        // `rename_all` produces `utf8`/`utf_8`; the protocol says `utf-8`, and
        // a profile document that disagreed with the wire would make the
        // negotiation assertion compare two different vocabularies.
        for enc in Encoding::all() {
            let json = serde_json::to_string(&enc).unwrap();
            assert_eq!(json, format!("\"{}\"", enc.as_str()));
            assert_eq!(serde_json::from_str::<Encoding>(&json).unwrap(), enc);
        }
    }
}
