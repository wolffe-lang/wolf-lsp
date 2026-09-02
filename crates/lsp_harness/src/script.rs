//! The session DSL — a scripted LSP session, committed beside its transcript.
//!
//! Sprint §1 asks for this by name, and the reason is re-recording. A
//! transcript is a *derived* artifact: when the server legitimately changes
//! what it answers, the fix is to re-run the session and review the diff, not
//! to hand-edit forty JSON lines into agreement. That only works if the
//! session itself is a committed input, so every transcript in this repo has a
//! `.lsps` beside it and `lspconf record` is the one-liner that regenerates
//! one.
//!
//! It is deliberately not a programming language. There are no variables, no
//! conditionals, and no expressions — a script that can compute is a script
//! whose behavior is not visible in its text, and the whole point of committing
//! it is that a reviewer can read what the session claims.
//!
//! ```text
//! name      smoke/open-hover
//! profile   minimal
//!
//! initialize
//! open      hello.lu
//! wait      diagnostics hello.lu
//! req       hover hello.lu 9:8
//! req       documentSymbol hello.lu
//! shutdown
//! ```
//!
//! Positions are written `line:character` **in the negotiated encoding**,
//! zero-based, exactly as they go on the wire. Writing them as byte offsets
//! would be friendlier to author and would hide the one thing §5 exists to
//! test.

use std::fmt;
use std::path::PathBuf;

use serde_json::Value;

/// One request the script issues.
#[derive(Debug, Clone, PartialEq)]
pub enum Req {
    Hover {
        file: String,
        line: u32,
        character: u32,
    },
    DocumentSymbol {
        file: String,
    },
    Formatting {
        file: String,
    },
    CodeAction {
        file: String,
        start: (u32, u32),
        end: (u32, u32),
    },
    /// s133 — the navigation trio, each a position request.
    Definition {
        file: String,
        line: u32,
        character: u32,
    },
    /// `includeDeclaration` is spelled: a references claim is a claim
    /// about a SET, and the declaration is in or out of it.
    References {
        file: String,
        line: u32,
        character: u32,
        include_declaration: bool,
    },
    PrepareRename {
        file: String,
        line: u32,
        character: u32,
    },
    Rename {
        file: String,
        line: u32,
        character: u32,
        new_name: String,
    },
    /// Escape hatch: any method, params written inline as JSON. How the
    /// unknown-method and pre-`initialize` obligations get exercised without
    /// teaching the DSL a verb per protocol method.
    Raw {
        method: String,
        params: Value,
    },
}

impl Req {
    /// The LSP method this issues.
    #[must_use]
    pub fn method(&self) -> &str {
        match self {
            Req::Hover { .. } => "textDocument/hover",
            Req::DocumentSymbol { .. } => "textDocument/documentSymbol",
            Req::Formatting { .. } => "textDocument/formatting",
            Req::CodeAction { .. } => "textDocument/codeAction",
            Req::Definition { .. } => "textDocument/definition",
            Req::References { .. } => "textDocument/references",
            Req::PrepareRename { .. } => "textDocument/prepareRename",
            Req::Rename { .. } => "textDocument/rename",
            Req::Raw { method, .. } => method,
        }
    }

    /// The document this request is about, if any.
    #[must_use]
    pub fn file(&self) -> Option<&str> {
        match self {
            Req::Hover { file, .. }
            | Req::DocumentSymbol { file }
            | Req::Formatting { file }
            | Req::CodeAction { file, .. }
            | Req::Definition { file, .. }
            | Req::References { file, .. }
            | Req::PrepareRename { file, .. }
            | Req::Rename { file, .. } => Some(file),
            Req::Raw { .. } => None,
        }
    }
}

/// One step of a session.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// `initialize` + `initialized`, with the script's profile.
    Initialize,
    /// `didOpen` with `source`'s bytes at `file`'s URI. `source` differs from
    /// `file` only for the local fixtures that fill a recorded corpus gap.
    Open {
        file: String,
        source: String,
    },
    /// `didChange` with a whole new text (full sync is what v0 negotiates).
    Edit {
        file: String,
        text: String,
        version: Option<i64>,
    },
    /// `didChange` whose text is the current text with `[lo, hi)` replaced —
    /// byte offsets, because that is how an edit is actually described and
    /// converting it to a position would test the harness's arithmetic.
    Splice {
        file: String,
        lo: usize,
        hi: usize,
        text: String,
    },
    /// `didChange` with `source`'s bytes.
    EditFrom {
        file: String,
        source: String,
    },
    Save {
        file: String,
        include_text: bool,
    },
    Close {
        file: String,
    },
    /// Send a request and wait for its response.
    Request(Req),
    /// Send a request and move on. The half of the DSL that makes
    /// cancellation and superseded-request scenarios expressible at all.
    Send(Req),
    /// `$/cancelRequest` for a request id.
    Cancel {
        id: i64,
    },
    /// Wait for `publishDiagnostics` on a document.
    WaitDiagnostics {
        file: String,
    },
    /// Wait for a response already sent by [`Step::Send`].
    WaitResponse {
        id: i64,
    },
    /// Collect whatever arrives for a window and then stop. For claims about
    /// *absence*, which cannot be waited for — only waited out.
    WaitQuiet {
        ms: u64,
    },
    Sleep {
        ms: u64,
    },
    /// `shutdown`, then `exit`, then wait for the process to leave.
    ShutdownExit,
    /// `shutdown` alone.
    Shutdown,
    /// `exit` alone — the "without shutdown" case, which the spec says exits
    /// nonzero.
    Exit,
    /// Verbatim bytes onto the wire, framing and all.
    Raw {
        bytes: Vec<u8>,
    },
}

/// A parsed script: its header plus its steps.
#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    /// `<client>/<scenario>`, the transcript's name.
    pub name: String,
    /// Capability profile under `profiles/`.
    pub profile: String,
    /// Workspace root, repo-relative.
    pub workspace: String,
    /// Free-text purpose, from `#!` lines. Carried into no artifact — it is
    /// there so the script explains itself to whoever re-records it.
    pub about: Vec<String>,
    /// Environment the server is spawned with.
    ///
    /// Spawn-time state belongs to the script rather than the transcript: the
    /// transcript is derived output and its format is frozen, while the script
    /// is the committed input a re-record reads. `replay` picks these up from
    /// the `.lsps` beside the `.jsonl` for exactly that reason.
    pub env: Vec<(String, String)>,
    pub steps: Vec<Step>,
}

/// A parse failure, located.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for Error {}

/// Default workspace for a script that does not name one.
pub const DEFAULT_WORKSPACE: &str = "vendor/upstream/samples";

impl Script {
    /// Parse a `.lsps` script.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut name = None;
        let mut profile = None;
        let mut workspace = None;
        let mut about = Vec::new();
        let mut env: Vec<(String, String)> = Vec::new();
        let mut steps = Vec::new();
        // Request ids are implicit and sequential in send order, starting at
        // 1 for `initialize` — so the FIRST `req` or `send` in a script is
        // id 2, not id 1. `cancel` and `wait response` name those numbers
        // directly, which is what makes them readable in the transcript, and
        // is also the one place an author reliably goes off by one.
        let mut next_id: i64 = 1;

        for (n, raw) in text.lines().enumerate() {
            let n = n + 1;
            let line = raw.trim_end();
            if let Some(note) = line.trim_start().strip_prefix("#!") {
                about.push(note.trim().to_string());
                continue;
            }
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (verb, rest) = split_word(line);
            let err = |m: String| Error {
                line: n,
                message: m,
            };
            match verb {
                "name" => name = Some(rest.to_string()),
                "profile" => profile = Some(rest.to_string()),
                "workspace" => workspace = Some(rest.to_string()),
                "env" => {
                    let (key, value) = split_word(rest);
                    if key.is_empty() {
                        return Err(err("`env` needs a NAME and a value".to_string()));
                    }
                    env.push((key.to_string(), value.to_string()));
                }
                "initialize" => {
                    steps.push(Step::Initialize);
                    next_id += 1;
                }
                "open" => {
                    let (file, src) = split_word(rest);
                    if file.is_empty() {
                        return Err(err("`open` needs a file".to_string()));
                    }
                    let source = if src.is_empty() { file } else { src };
                    steps.push(Step::Open {
                        file: file.to_string(),
                        source: source.to_string(),
                    });
                }
                "edit" => {
                    let (file, json) = split_word(rest);
                    let text = parse_json_string(json).map_err(&err)?;
                    steps.push(Step::Edit {
                        file: file.to_string(),
                        text,
                        version: None,
                    });
                }
                "edit-version" => {
                    let (file, rest) = split_word(rest);
                    let (ver, json) = split_word(rest);
                    let version: i64 = ver
                        .parse()
                        .map_err(|_| err(format!("`edit-version` wants a number, got {ver:?}")))?;
                    let text = parse_json_string(json).map_err(&err)?;
                    steps.push(Step::Edit {
                        file: file.to_string(),
                        text,
                        version: Some(version),
                    });
                }
                "edit-from" => {
                    let (file, source) = split_word(rest);
                    if source.is_empty() {
                        return Err(err("`edit-from` needs a file and a source".to_string()));
                    }
                    steps.push(Step::EditFrom {
                        file: file.to_string(),
                        source: source.to_string(),
                    });
                }
                "splice" => {
                    let (file, rest) = split_word(rest);
                    let (lo, rest) = split_word(rest);
                    let (hi, json) = split_word(rest);
                    let lo = lo
                        .parse()
                        .map_err(|_| err(format!("`splice` lo is not a number: {lo:?}")))?;
                    let hi = hi
                        .parse()
                        .map_err(|_| err(format!("`splice` hi is not a number: {hi:?}")))?;
                    let text = parse_json_string(json).map_err(&err)?;
                    steps.push(Step::Splice {
                        file: file.to_string(),
                        lo,
                        hi,
                        text,
                    });
                }
                "save" => {
                    let (file, flag) = split_word(rest);
                    steps.push(Step::Save {
                        file: file.to_string(),
                        include_text: flag == "with-text",
                    });
                }
                "close" => steps.push(Step::Close {
                    file: rest.to_string(),
                }),
                "req" | "send" => {
                    let req = parse_req(rest).map_err(&err)?;
                    steps.push(if verb == "req" {
                        Step::Request(req)
                    } else {
                        Step::Send(req)
                    });
                    next_id += 1;
                }
                "cancel" => {
                    let id = rest
                        .parse()
                        .map_err(|_| err(format!("`cancel` wants a request id, got {rest:?}")))?;
                    steps.push(Step::Cancel { id });
                }
                "wait" => {
                    let (what, arg) = split_word(rest);
                    steps.push(match what {
                        "diagnostics" => Step::WaitDiagnostics {
                            file: arg.to_string(),
                        },
                        "response" => Step::WaitResponse {
                            id: arg.parse().map_err(|_| {
                                err(format!("`wait response` wants an id, got {arg:?}"))
                            })?,
                        },
                        "quiet" => Step::WaitQuiet {
                            ms: arg
                                .parse()
                                .map_err(|_| err(format!("`wait quiet` wants ms, got {arg:?}")))?,
                        },
                        other => {
                            return Err(err(format!(
                                "unknown `wait` kind `{other}` \
                                 — expected diagnostics, response, or quiet"
                            )));
                        }
                    });
                }
                "sleep" => steps.push(Step::Sleep {
                    ms: rest
                        .parse()
                        .map_err(|_| err(format!("`sleep` wants ms, got {rest:?}")))?,
                }),
                "shutdown" => steps.push(Step::ShutdownExit),
                "shutdown-only" => steps.push(Step::Shutdown),
                "exit" => steps.push(Step::Exit),
                "raw" => {
                    let bytes = parse_json_string(rest).map_err(&err)?.into_bytes();
                    steps.push(Step::Raw { bytes });
                }
                other => {
                    return Err(err(format!(
                        "unknown verb `{other}` — expected one of: name, profile, workspace, \
                         env, initialize, open, edit, edit-version, edit-from, splice, save, \
                         close, req, send, cancel, wait, sleep, shutdown, shutdown-only, exit, raw"
                    )));
                }
            }
        }

        let _ = next_id;
        Ok(Self {
            name: name.ok_or(Error {
                line: 1,
                message: "the script has no `name` — a transcript without one cannot be filed"
                    .to_string(),
            })?,
            profile: profile.ok_or(Error {
                line: 1,
                message: "the script has no `profile` — capability negotiation is not optional, \
                          and `minimal` is a real answer"
                    .to_string(),
            })?,
            workspace: workspace.unwrap_or_else(|| DEFAULT_WORKSPACE.to_string()),
            about,
            env,
            steps,
        })
    }

    /// The transcript file this script records into: same stem, `.jsonl`.
    #[must_use]
    pub fn transcript_path(script_path: &std::path::Path) -> PathBuf {
        script_path.with_extension("jsonl")
    }
}

fn split_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i..].trim_start()),
        None => (s, ""),
    }
}

/// Read a JSON string literal, so scripts can carry newlines, tabs, and
/// astral-plane text on one line without inventing a second escape language.
fn parse_json_string(s: &str) -> Result<String, String> {
    let s = s.trim();
    match serde_json::from_str::<Value>(s) {
        Ok(Value::String(text)) => Ok(text),
        Ok(_) => Err(format!("expected a JSON string literal, got {s}")),
        Err(e) => Err(format!("expected a JSON string literal: {e}")),
    }
}

fn parse_pos(s: &str) -> Result<(u32, u32), String> {
    let (l, c) = s
        .split_once(':')
        .ok_or_else(|| format!("expected `line:character`, got {s:?}"))?;
    Ok((
        l.parse().map_err(|_| format!("bad line {l:?}"))?,
        c.parse().map_err(|_| format!("bad character {c:?}"))?,
    ))
}

fn parse_req(rest: &str) -> Result<Req, String> {
    let (kind, args) = split_word(rest);
    Ok(match kind {
        "hover" => {
            let (file, pos) = split_word(args);
            let (line, character) = parse_pos(pos)?;
            Req::Hover {
                file: file.to_string(),
                line,
                character,
            }
        }
        "documentSymbol" => Req::DocumentSymbol {
            file: args.trim().to_string(),
        },
        "formatting" => Req::Formatting {
            file: args.trim().to_string(),
        },
        "codeAction" => {
            let (file, range) = split_word(args);
            let (s, e) = range
                .split_once('-')
                .ok_or_else(|| format!("expected `l:c-l:c`, got {range:?}"))?;
            Req::CodeAction {
                file: file.to_string(),
                start: parse_pos(s)?,
                end: parse_pos(e)?,
            }
        }
        "definition" => {
            let (file, pos) = split_word(args);
            let (line, character) = parse_pos(pos)?;
            Req::Definition {
                file: file.to_string(),
                line,
                character,
            }
        }
        // `req references <file> <l:c> [decl|nodecl]` — the declaration's
        // membership is part of the claim, so the script says it.
        "references" => {
            let (file, rest) = split_word(args);
            let (pos, flag) = split_word(rest);
            let (line, character) = parse_pos(pos)?;
            let include_declaration = match flag.trim() {
                "" | "nodecl" => false,
                "decl" => true,
                other => {
                    return Err(format!(
                        "expected `decl` or `nodecl` after the position, got {other:?}"
                    ));
                }
            };
            Req::References {
                file: file.to_string(),
                line,
                character,
                include_declaration,
            }
        }
        "prepareRename" => {
            let (file, pos) = split_word(args);
            let (line, character) = parse_pos(pos)?;
            Req::PrepareRename {
                file: file.to_string(),
                line,
                character,
            }
        }
        // `req rename <file> <l:c> <newName>`.
        "rename" => {
            let (file, rest) = split_word(args);
            let (pos, new_name) = split_word(rest);
            let (line, character) = parse_pos(pos)?;
            let new_name = new_name.trim();
            if new_name.is_empty() {
                return Err("`req rename` needs a new name after the position".to_string());
            }
            Req::Rename {
                file: file.to_string(),
                line,
                character,
                new_name: new_name.to_string(),
            }
        }
        "raw" => {
            let (method, params) = split_word(args);
            let params: Value = serde_json::from_str(params.trim())
                .map_err(|e| format!("`req raw` params are not JSON: {e}"))?;
            Req::Raw {
                method: method.to_string(),
                params,
            }
        }
        other => {
            return Err(format!(
                "unknown request kind `{other}` — expected hover, documentSymbol, \
                 formatting, codeAction, definition, references, prepareRename, \
                 rename, or raw"
            ));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
#! why this session exists
name      smoke/open-hover
profile   minimal

initialize
open      hello.lu
wait      diagnostics hello.lu
req       hover hello.lu 9:8
send      documentSymbol hello.lu
cancel    3
wait      response 3
shutdown
"#;

    #[test]
    fn parses_the_committed_shape() {
        let s = Script::parse(SAMPLE).unwrap();
        assert_eq!(s.name, "smoke/open-hover");
        assert_eq!(s.profile, "minimal");
        assert_eq!(s.workspace, DEFAULT_WORKSPACE);
        assert_eq!(s.about, vec!["why this session exists"]);
        assert_eq!(s.steps.len(), 8);
        assert_eq!(
            s.steps[3],
            Step::Request(Req::Hover {
                file: "hello.lu".to_string(),
                line: 9,
                character: 8
            })
        );
        assert_eq!(s.steps[5], Step::Cancel { id: 3 });
    }

    #[test]
    fn env_is_a_header_directive_because_it_is_spawn_time_state() {
        let s =
            Script::parse("name a/b\nprofile p\nenv WOLF_QUERY_TEST_SLOW_MS 1200\ninitialize\n")
                .unwrap();
        assert_eq!(
            s.env,
            vec![("WOLF_QUERY_TEST_SLOW_MS".to_string(), "1200".to_string())]
        );
        assert_eq!(s.steps, vec![Step::Initialize]);
    }

    #[test]
    fn a_script_without_a_profile_is_refused() {
        // Not a default: "which capabilities did the client declare?" has no
        // safe guess, and `minimal` is a real answer someone must choose.
        let err = Script::parse("name x/y\ninitialize\n").unwrap_err();
        assert!(err.message.contains("profile"), "{err}");
    }

    #[test]
    fn an_unknown_verb_lists_the_known_ones() {
        let err = Script::parse("name a/b\nprofile minimal\nhoverr x\n").unwrap_err();
        assert_eq!(err.line, 3);
        assert!(err.message.contains("unknown verb"), "{err}");
        assert!(err.message.contains("documentSymbol") || err.message.contains("req"));
    }

    #[test]
    fn edit_text_is_a_json_string_so_it_can_carry_newlines_and_astral_text() {
        let s = Script::parse(
            "name a/b\nprofile minimal\nedit a.lu \"fn main() {\\n\\t\\\"\\ud83d\\udc3a\\\"\\n}\"\n",
        )
        .unwrap();
        let Step::Edit { text, .. } = &s.steps[0] else {
            panic!("{:?}", s.steps)
        };
        assert!(text.contains('\n') && text.contains('\t') && text.contains('\u{1F43A}'));
    }

    #[test]
    fn a_splice_carries_byte_offsets_not_positions() {
        let s = Script::parse("name a/b\nprofile minimal\nsplice a.lu 10 14 \"xy\"\n").unwrap();
        assert_eq!(
            s.steps[0],
            Step::Splice {
                file: "a.lu".to_string(),
                lo: 10,
                hi: 14,
                text: "xy".to_string()
            }
        );
    }

    #[test]
    fn comments_and_blank_lines_are_ignored_but_bang_comments_are_kept() {
        let s =
            Script::parse("#! purpose\n# noise\nname a/b\nprofile p\n\ninitialize # trailing\n")
                .unwrap();
        assert_eq!(s.about, vec!["purpose"]);
        assert_eq!(s.steps, vec![Step::Initialize]);
    }

    #[test]
    fn open_as_lets_a_local_fixture_stand_in_for_a_corpus_path() {
        let s = Script::parse("name a/b\nprofile p\nopen astral.lu ../../../fixtures/astral.lu\n")
            .unwrap();
        assert_eq!(
            s.steps[0],
            Step::Open {
                file: "astral.lu".to_string(),
                source: "../../../fixtures/astral.lu".to_string()
            }
        );
    }
}
