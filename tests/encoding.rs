//! §5 — `positionEncoding` correctness, in **both directions**, under every
//! negotiated encoding.
//!
//! The transcripts under `transcripts/encoding/` pin *what* the server answers.
//! This file asserts *why* it is right: every position the server produces is
//! recomputed here with [`lsp_transcript::encoding`], an implementation written
//! from the specification and deliberately not the server's. Two independent
//! implementations agreeing is evidence; one implementation agreeing with
//! itself is not.
//!
//! The two directions, in the sprint's words:
//!
//! - **byte span → LSP range** — a diagnostic on astral text lands on the
//!   intended characters. Asserted by converting the compiler's own byte span
//!   (from `wolf conform-run --error-format=json`) and comparing.
//! - **client position → offset** — hover after an emoji resolves the right
//!   token. Asserted by feeding the server a position computed here for a
//!   known byte offset and checking the token it answers about.

mod support;

use std::time::Instant;

use lsp_transcript::encoding::{self, Encoding, LineIndex, Position};
use serde_json::{Value, json};

/// Open the astral fixture under one encoding and hand back everything the
/// assertions need.
struct Fixture {
    session: lsp_harness::Session,
    uri: String,
    bytes: Vec<u8>,
    index: LineIndex,
    enc: Encoding,
}

fn open(server: &support::Server, profile: &str) -> Fixture {
    let workspace = server.fixtures();
    let profile = server.profile(profile);
    let mut session = server.session(&workspace, &profile);
    let bytes = support::read(&workspace, "astral.lu");
    let uri = lsp_harness::session::file_uri(&workspace.join("astral.lu"));
    session
        .notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "wolf", "version": 1,
                                    "text": String::from_utf8_lossy(&bytes)}}),
        )
        .expect("didOpen");
    session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish");
    let enc = session.encoding();
    Fixture {
        session,
        uri,
        index: LineIndex::new(&bytes),
        bytes,
        enc,
    }
}

impl Fixture {
    fn hover(&mut self, pos: Position) -> Value {
        let started = Instant::now();
        let id = self
            .session
            .request(
                "textDocument/hover",
                json!({"textDocument": {"uri": self.uri},
                       "position": {"line": pos.line, "character": pos.character}}),
            )
            .expect("hover");
        self.session
            .response(id, started)
            .expect("hover answered")
            .0
    }

    /// The byte offset the server resolved a hover to, read back out of the
    /// range it reported.
    fn hover_span(&mut self, pos: Position) -> Option<(u32, u32)> {
        let resp = self.hover(pos);
        let r = resp.pointer("/result/range")?;
        let at = |which: &str| -> Position {
            Position::new(
                r.pointer(&format!("/{which}/line"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                r.pointer(&format!("/{which}/character"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
            )
        };
        Some((
            encoding::position_to_offset(&self.bytes, &self.index, at("start"), self.enc),
            encoding::position_to_offset(&self.bytes, &self.index, at("end"), self.enc),
        ))
    }

    /// The position a byte offset occupies, computed independently of the
    /// server.
    fn position_of(&self, offset: u32) -> Position {
        encoding::offset_to_position(&self.bytes, &self.index, offset, self.enc)
    }

    fn finish(mut self) {
        self.session.shutdown_exit().expect("clean shutdown");
    }
}

/// The byte offset of the `n`-th occurrence of `needle`.
fn find(bytes: &[u8], needle: &str) -> u32 {
    let hay = std::str::from_utf8(bytes).expect("the fixture is valid UTF-8");
    hay.find(needle)
        .unwrap_or_else(|| panic!("the fixture no longer contains {needle:?}")) as u32
}

fn rfind(bytes: &[u8], needle: &str) -> u32 {
    let hay = std::str::from_utf8(bytes).expect("the fixture is valid UTF-8");
    hay.rfind(needle)
        .unwrap_or_else(|| panic!("the fixture no longer contains {needle:?}")) as u32
}

const PROFILES: [(&str, Encoding); 3] = [
    ("utf8-first", Encoding::Utf8),
    ("utf16-only", Encoding::Utf16),
    ("utf32-only", Encoding::Utf32),
];

#[test]
fn a_position_computed_here_resolves_to_the_token_it_names() {
    // client position → offset. `bmp` sits at the end of a line that begins
    // with astral, BMP, combining and ZWJ text; the position handed to the
    // server is computed from its byte offset by this repo's own conversion,
    // and the server must answer about that identifier and no other.
    let Some(server) = support::server() else {
        return;
    };
    for (profile, expected) in PROFILES {
        let mut fx = open(&server, profile);
        assert_eq!(fx.enc, expected, "profile `{profile}`");

        let target = rfind(&fx.bytes, "bmp");
        let pos = fx.position_of(target);
        let span = fx
            .hover_span(pos)
            .unwrap_or_else(|| panic!("{profile}: hover at {pos} answered nothing"));
        assert_eq!(
            span.0, target,
            "{profile}: hover at {pos} resolved to byte {} rather than {target}",
            span.0
        );
        assert_eq!(
            &fx.bytes[span.0 as usize..span.1 as usize],
            b"bmp",
            "{profile}: hover covered the wrong bytes"
        );
        fx.finish();
    }
}

#[test]
fn the_same_token_is_a_different_column_in_each_encoding() {
    // The negative control for the test above. If all three encodings produced
    // the same `character`, the conversion would be doing nothing and every
    // other assertion in this file would pass vacuously.
    let Some(server) = support::server() else {
        return;
    };
    let mut columns = Vec::new();
    for (profile, _) in PROFILES {
        let fx = open(&server, profile);
        let target = rfind(&fx.bytes, "bmp");
        columns.push(fx.position_of(target).character);
        fx.finish();
    }
    let [u8c, u16c, u32c] = <[u32; 3]>::try_from(columns.as_slice()).expect("three encodings");
    assert!(
        u8c > u16c && u16c > u32c,
        "bytes > UTF-16 units > code points must hold on this line: {u8c}, {u16c}, {u32c}"
    );
}

#[test]
fn a_position_inside_a_surrogate_pair_is_defined_and_stable() {
    // The illegal case. Under utf-16 the second unit of an astral code point
    // is not addressable; the spec's answer is that it resolves to the code
    // point's start, and the claim being pinned is that the answer is *stable*
    // — the same request twice must not drift, and it must not error.
    let Some(server) = support::server() else {
        return;
    };
    let mut fx = open(&server, "utf16-only");
    let emoji = find(&fx.bytes, "\u{1F43A}");
    let at_start = fx.position_of(emoji);
    let interior = Position::new(at_start.line, at_start.character + 1);

    let first = fx.hover(interior);
    let second = fx.hover(interior);
    assert_eq!(
        first.pointer("/result"),
        second.pointer("/result"),
        "the interior of a surrogate pair answered differently on two identical requests"
    );
    assert!(
        first.get("error").is_none(),
        "an unaddressable position must resolve, not error: {first}"
    );
    // And the harness's own conversion agrees on where it lands.
    assert_eq!(
        encoding::position_to_offset(&fx.bytes, &fx.index, interior, Encoding::Utf16),
        emoji,
        "an interior position must resolve to the code point's start"
    );
    fx.finish();
}

#[test]
fn a_diagnostic_span_after_astral_text_lands_on_the_intended_characters() {
    // byte span → LSP range, end to end. The overlay introduces a chained
    // comparison at a byte offset this test chooses, on a line that already
    // holds astral, BMP, combining and ZWJ text — then the published range is
    // converted back to bytes here and must be exactly the offset that was
    // broken.
    let Some(server) = support::server() else {
        return;
    };
    for (profile, _) in PROFILES {
        let mut fx = open(&server, profile);
        let target = rfind(&fx.bytes, "bmp");
        let edited = lsp_harness::drive::splice(
            &fx.bytes,
            target as usize,
            target as usize + 3,
            b"1 < 2 < 3",
        );
        fx.session
            .notify(
                "textDocument/didChange",
                json!({"textDocument": {"uri": fx.uri, "version": 2},
                       "contentChanges": [{"text": String::from_utf8_lossy(&edited)}]}),
            )
            .expect("didChange");
        let (published, _) = fx
            .session
            .notification(
                "textDocument/publishDiagnostics",
                Some(&fx.uri),
                Instant::now(),
            )
            .expect("publish after edit");

        let diagnostics = published
            .pointer("/params/diagnostics")
            .and_then(Value::as_array)
            .expect("diagnostics");
        assert_eq!(diagnostics.len(), 1, "{profile}: {published}");
        assert_eq!(diagnostics[0]["code"], "E0003", "{profile}");

        // Convert the published range back to bytes with *this* repo's
        // conversion against the *edited* buffer, and check the offset is
        // inside the text the edit introduced.
        let index = LineIndex::new(&edited);
        let start = Position::new(
            diagnostics[0]["range"]["start"]["line"].as_u64().unwrap() as u32,
            diagnostics[0]["range"]["start"]["character"]
                .as_u64()
                .unwrap() as u32,
        );
        let offset = encoding::position_to_offset(&edited, &index, start, fx.enc);
        assert!(
            offset >= target && offset < target + 9,
            "{profile}: E0003 published at {start} = byte {offset}, but the edit that \
             caused it spans bytes {target}..{}",
            target + 9
        );
        // The character under the reported span is the `<` that chained.
        assert_eq!(
            edited.get(offset as usize).copied(),
            Some(b'<'),
            "{profile}: the span points at {:?}, not the offending operator",
            edited.get(offset as usize).map(|b| *b as char)
        );
        fx.finish();
    }
}

#[test]
fn crlf_content_does_not_shift_a_single_column() {
    // CRLF arrives as `didChange` content — there is no CRLF file in this repo
    // and `.gitattributes` guarantees there never will be. The server splits
    // lines on `\n` alone, so a lone `\r` is ordinary line content; the claim
    // is that a diagnostic on line 1 of a CRLF buffer reports the same line
    // and column as the same diagnostic in the LF buffer.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("utf16-only");
    let mut session = server.session(&workspace, &profile);
    let uri = lsp_harness::session::file_uri(&workspace.join("hello.lu"));
    let original = String::from_utf8_lossy(&support::read(&workspace, "hello.lu")).into_owned();
    session
        .notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "wolf",
                                    "version": 1, "text": original}}),
        )
        .expect("didOpen");
    session
        .notification(
            "textDocument/publishDiagnostics",
            Some(&uri),
            Instant::now(),
        )
        .expect("publish");

    let lf = "fn main() -> !int {\n    let a = 1 < 2 < 3\n    0\n}\n";
    let crlf = lf.replace('\n', "\r\n");
    let mut ranges = Vec::new();
    for (version, text) in [(2, lf.to_string()), (3, crlf)] {
        session
            .notify(
                "textDocument/didChange",
                json!({"textDocument": {"uri": uri, "version": version},
                       "contentChanges": [{"text": text}]}),
            )
            .expect("didChange");
        let (published, _) = session
            .notification(
                "textDocument/publishDiagnostics",
                Some(&uri),
                Instant::now(),
            )
            .expect("publish");
        let diagnostics = published
            .pointer("/params/diagnostics")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(diagnostics.len(), 1, "{published}");
        assert_eq!(diagnostics[0]["code"], "E0003");
        ranges.push(diagnostics[0]["range"].clone());
    }
    assert_eq!(
        ranges[0], ranges[1],
        "the same diagnostic moved when the buffer's line endings changed — \
         a CRLF user would see every squiggle in the wrong place"
    );
    session.shutdown_exit().expect("clean shutdown");
}

#[test]
fn a_tab_is_one_code_unit_and_not_a_screen_column() {
    // Nothing in LSP knows about tab stops, and a server that expanded tabs
    // when computing a column would put every squiggle on a tab-indented line
    // in the wrong place. The fixture has a literal tab inside a string.
    let Some(server) = support::server() else {
        return;
    };
    for (profile, _) in PROFILES {
        let fx = open(&server, profile);
        let tab = find(&fx.bytes, "left\tright") + 4;
        assert_eq!(fx.bytes[tab as usize], b'\t');
        let before = fx.position_of(tab);
        let after = fx.position_of(tab + 1);
        assert_eq!(
            after.character - before.character,
            1,
            "{profile}: a tab occupied {} units",
            after.character - before.character
        );
        fx.finish();
    }
}

#[test]
fn a_very_long_line_converts_correctly_at_its_far_end() {
    // Past any plausible small-buffer boundary, and with an astral character
    // at the very end so the last conversion on the line is the hard one.
    let Some(server) = support::server() else {
        return;
    };
    for (profile, enc) in PROFILES {
        let fx = open(&server, profile);
        let long = find(&fx.bytes, "the wolf runs and the moon watches");
        let line = fx.index.line_of(long);
        let (start, end) = fx.index.line_range(line);
        assert!(end - start > 400, "{profile}: the long line got short");

        // Every code-point boundary on that line round-trips.
        let mut off = start;
        while off < end {
            let pos = fx.position_of(off);
            assert_eq!(
                encoding::position_to_offset(&fx.bytes, &fx.index, pos, enc),
                off,
                "{profile}: byte {off} on the long line did not round-trip"
            );
            off += encoding::seq_len(fx.bytes[off as usize]) as u32;
        }
        fx.finish();
    }
}
