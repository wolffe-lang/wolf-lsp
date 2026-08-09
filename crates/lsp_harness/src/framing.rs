//! `Content-Length` framing for JSON-RPC over stdio — **ours on purpose**.
//!
//! Owning ~150 lines here rather than taking a crate is a testing decision
//! (ls00 §7): ls01 has to emit *deliberately malformed* frames — a bad
//! `Content-Length`, a split header, a body that stops early — and a library
//! that only produces well-formed output cannot exercise a server's parser at
//! all. [`write_raw`] is that escape hatch, and it exists from day one so the
//! ls01 tests do not have to reach around this module.
//!
//! The reader is written against the base protocol's actual rules: headers are
//! ASCII, `\r\n`-terminated, ended by a blank line; `Content-Length` is
//! mandatory; `Content-Type` is optional and its charset is `utf-8`. Unknown
//! headers are skipped, because the spec allows them and a client that dies on
//! one is a client that dies on the next protocol revision.

use std::fmt;
use std::io::{self, BufRead, Write};

/// Refuse to allocate more than this for one message.
///
/// A malformed `Content-Length` is a *likely* input here — this harness
/// generates them — so the reader must fail loudly rather than try to allocate
/// whatever number it was handed.
pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

/// Why a frame could not be read.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    /// A header line that is not `Name: value`.
    MalformedHeader(String),
    /// Headers ended without a `Content-Length`.
    MissingContentLength,
    /// `Content-Length` was present but not a plausible length.
    BadContentLength(String),
    /// `Content-Type` named a charset this transport does not speak.
    UnsupportedCharset(String),
    /// The stream ended mid-body.
    Truncated {
        expected: usize,
        got: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::MalformedHeader(l) => write!(f, "malformed header line: {l:?}"),
            Error::MissingContentLength => f.write_str("headers ended without a Content-Length"),
            Error::BadContentLength(v) => write!(f, "bad Content-Length: {v:?}"),
            Error::UnsupportedCharset(c) => {
                write!(
                    f,
                    "Content-Type names charset {c:?}; this transport is utf-8 only"
                )
            }
            Error::Truncated { expected, got } => {
                write!(f, "stream ended after {got} of {expected} body bytes")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

/// Reads framed messages off a stream.
pub struct FrameReader<R: BufRead> {
    inner: R,
}

impl<R: BufRead> FrameReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Read one message body. `Ok(None)` is a clean end of stream — which is
    /// what a well-behaved server does after `exit`, and must never be
    /// reported as an error.
    pub fn read_frame(&mut self) -> Result<Option<Vec<u8>>, Error> {
        let mut content_length: Option<usize> = None;
        let mut saw_any_header = false;

        loop {
            let mut line = String::new();
            let n = self.inner.read_line(&mut line)?;
            if n == 0 {
                return if saw_any_header {
                    Err(Error::Truncated {
                        expected: 0,
                        got: 0,
                    })
                } else {
                    Ok(None)
                };
            }
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }
            saw_any_header = true;

            let Some((name, value)) = line.split_once(':') else {
                return Err(Error::MalformedHeader(line.to_string()));
            };
            let value = value.trim();
            // Header names are case-insensitive; clients disagree about the
            // casing and at least one sends `content-length`.
            match name.trim().to_ascii_lowercase().as_str() {
                "content-length" => {
                    let len: usize = value
                        .parse()
                        .map_err(|_| Error::BadContentLength(value.to_string()))?;
                    if len > MAX_FRAME_BYTES {
                        return Err(Error::BadContentLength(value.to_string()));
                    }
                    content_length = Some(len);
                }
                "content-type" => {
                    if let Some((_, charset)) = value.split_once("charset=") {
                        let charset = charset.trim().trim_matches('"').to_ascii_lowercase();
                        if charset != "utf-8" && charset != "utf8" {
                            return Err(Error::UnsupportedCharset(charset));
                        }
                    }
                }
                _ => {} // Unknown headers are permitted; skip.
            }
        }

        let len = content_length.ok_or(Error::MissingContentLength)?;
        let mut body = vec![0u8; len];
        match self.inner.read_exact(&mut body) {
            Ok(()) => Ok(Some(body)),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Err(Error::Truncated {
                expected: len,
                got: 0,
            }),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Surrender the underlying reader (ls01 needs it to drain stderr on a
    /// failure without losing buffered bytes).
    pub fn into_inner(self) -> R {
        self.inner
    }
}

/// Write one well-formed frame.
///
/// `Content-Length` counts **bytes, not characters** — the single most common
/// framing bug in hand-written clients, and the reason this is a function
/// rather than a format string at each call site.
pub fn write_frame<W: Write>(w: &mut W, body: &[u8]) -> io::Result<()> {
    write!(w, "Content-Length: {}\r\n\r\n", body.len())?;
    w.write_all(body)?;
    w.flush()
}

/// Write bytes verbatim, framing and all.
///
/// The whole reason this module is not a dependency: ls01's malformed-frame
/// suite needs to send a header with a length that lies, a body split across
/// two writes, a missing blank line. Nothing here validates; that is the point.
pub fn write_raw<W: Write>(w: &mut W, bytes: &[u8]) -> io::Result<()> {
    w.write_all(bytes)?;
    w.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_all(input: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
        let mut r = FrameReader::new(io::BufReader::new(input));
        let mut out = Vec::new();
        while let Some(frame) = r.read_frame()? {
            out.push(frame);
        }
        Ok(out)
    }

    #[test]
    fn round_trips_a_frame() {
        let mut buf = Vec::new();
        write_frame(&mut buf, br#"{"jsonrpc":"2.0"}"#).unwrap();
        assert_eq!(
            buf,
            b"Content-Length: 17\r\n\r\n{\"jsonrpc\":\"2.0\"}".to_vec()
        );
        assert_eq!(
            read_all(&buf).unwrap(),
            vec![br#"{"jsonrpc":"2.0"}"#.to_vec()]
        );
    }

    #[test]
    fn reads_two_frames_back_to_back() {
        let mut buf = Vec::new();
        write_frame(&mut buf, b"{}").unwrap();
        write_frame(&mut buf, b"[]").unwrap();
        assert_eq!(
            read_all(&buf).unwrap(),
            vec![b"{}".to_vec(), b"[]".to_vec()]
        );
    }

    #[test]
    fn content_length_counts_bytes_not_characters() {
        // Four bytes, one character, two UTF-16 units — the arithmetic the
        // whole ls01 encoding suite exists to police.
        let body = "\"\u{1F600}\"".as_bytes();
        let mut buf = Vec::new();
        write_frame(&mut buf, body).unwrap();
        assert!(buf.starts_with(b"Content-Length: 6\r\n"));
        assert_eq!(read_all(&buf).unwrap(), vec![body.to_vec()]);
    }

    #[test]
    fn header_names_are_case_insensitive_and_unknown_headers_are_skipped() {
        let input = b"content-length: 2\r\nX-Whatever: 1\r\n\r\n{}";
        assert_eq!(read_all(input).unwrap(), vec![b"{}".to_vec()]);
    }

    #[test]
    fn clean_eof_is_not_an_error() {
        assert_eq!(read_all(b"").unwrap(), Vec::<Vec<u8>>::new());
    }

    #[test]
    fn missing_content_length_is_loud() {
        let err = read_all(b"X-Only: 1\r\n\r\n{}").unwrap_err();
        assert!(matches!(err, Error::MissingContentLength), "{err}");
    }

    #[test]
    fn a_lying_content_length_is_truncation_not_a_hang() {
        let err = read_all(b"Content-Length: 99\r\n\r\n{}").unwrap_err();
        assert!(
            matches!(err, Error::Truncated { expected: 99, .. }),
            "{err}"
        );
    }

    #[test]
    fn an_absurd_content_length_is_refused_before_allocating() {
        let err = read_all(b"Content-Length: 999999999999\r\n\r\n").unwrap_err();
        assert!(matches!(err, Error::BadContentLength(_)), "{err}");
    }

    #[test]
    fn a_foreign_charset_is_refused() {
        let input = b"Content-Length: 2\r\nContent-Type: application/vscode-jsonrpc; charset=utf-16\r\n\r\n{}";
        let err = read_all(input).unwrap_err();
        assert!(matches!(err, Error::UnsupportedCharset(_)), "{err}");
    }

    #[test]
    fn malformed_header_lines_are_reported_with_the_line() {
        let err = read_all(b"not-a-header\r\n\r\n").unwrap_err();
        assert!(
            matches!(err, Error::MalformedHeader(ref l) if l == "not-a-header"),
            "{err}"
        );
    }
}
