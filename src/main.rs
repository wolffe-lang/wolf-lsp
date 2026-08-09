//! `lspconf` — record | replay | verify | doctor | bench.
//!
//! The harness front end. Two of these run today; three cannot, because the
//! server does not exist yet: `wolf lsp` is the compiler itself (D34) and the
//! subcommand lands with wolf-lang's s52. Rather than stub a server or fake a
//! pass, every server-dependent path asks [`lsp_harness::Doctor`] and, when the
//! answer is "no server at the pin", prints
//!
//! ```text
//! SKIP: no wolf binary at pin ecea37c (…)
//! ```
//!
//! and exits `77`. `--require-server` turns that skip into a failure, which is
//! how a CI job that is *supposed* to have a binary proves it had one.
//! Silence is what turns a dark lane into a lane nobody notices is dark.
//!
//! Exit codes (mirroring the compiler and interpreter tracks, plus 77):
//! `0` matched · `1` mismatch / required server missing · `2` harness error ·
//! `77` skipped.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lsp_harness::{Doctor, EXIT_HARNESS_ERROR, EXIT_MISMATCH, EXIT_OK, EXIT_SKIPPED};
use lsp_transcript::jsonl;

const USAGE: &str = "\
usage: lspconf [--require-server] <command>

  verify [path…]   parse, validate, and canonicalize transcripts and profiles
                   (server-free; this is the half of CI that always runs)
  doctor           report the pin, the binary that won resolution, and whether
                   a server is available
  record           capture a live session          (ls01; needs a server)
  replay <file>    drive a recorded session        (ls01; needs a server)
  bench            latency budgets, D5 JSONL shape (ls01; needs a server)

  --require-server treat an unavailable server as a failure instead of a skip
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let require_server = args.iter().any(|a| a == "--require-server");
    let positional: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| !a.starts_with("--"))
        .collect();

    let root = match repo_root() {
        Ok(r) => r,
        Err(msg) => return fail(&msg),
    };

    match positional.first().copied() {
        Some("verify") => verify(&root, &positional[1..]),
        Some("doctor") => doctor(&root, require_server),
        Some(cmd @ ("record" | "replay" | "bench")) => gated(&root, cmd, require_server),
        Some(other) => {
            eprintln!("lspconf: unknown command `{other}`\n\n{USAGE}");
            ExitCode::from(EXIT_HARNESS_ERROR as u8)
        }
        None => {
            eprintln!("{USAGE}");
            ExitCode::from(EXIT_HARNESS_ERROR as u8)
        }
    }
}

fn repo_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot read the cwd: {e}"))?;
    lsp_harness::find_repo_root(&cwd).ok_or_else(|| {
        format!(
            "not inside a wolf-lsp checkout (looked upward from {} for Cargo.toml + vendor/upstream)",
            cwd.display()
        )
    })
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("lspconf: {msg}");
    ExitCode::from(EXIT_HARNESS_ERROR as u8)
}

// ------------------------------------------------------------- doctor --

fn doctor(root: &Path, require_server: bool) -> ExitCode {
    let doc = Doctor::run(root);
    print!("{doc}");

    match doc.skip_line() {
        None => ExitCode::from(EXIT_OK as u8),
        Some(line) => {
            println!("{line}");
            if doc.availability.is_skip() {
                if require_server {
                    println!(
                        "FATAL: --require-server was given, so an unavailable server is a failure"
                    );
                    ExitCode::from(EXIT_MISMATCH as u8)
                } else {
                    ExitCode::from(EXIT_SKIPPED as u8)
                }
            } else {
                // A wrong binary or an unreadable pin is never a skip: testing
                // the wrong software is worse than testing none.
                ExitCode::from(EXIT_MISMATCH as u8)
            }
        }
    }
}

// -------------------------------------------------- server-dependent --

fn gated(root: &Path, cmd: &str, require_server: bool) -> ExitCode {
    let doc = Doctor::run(root);
    if let Some(line) = doc.skip_line() {
        println!("{line}");
        println!("SKIP: `lspconf {cmd}` needs a server");
        return if doc.availability.is_skip() && !require_server {
            ExitCode::from(EXIT_SKIPPED as u8)
        } else {
            ExitCode::from(EXIT_MISMATCH as u8)
        };
    }
    // A server exists and matches the pin — which means someone bumped past
    // wolf-lang's s52 and this command now has to be written.
    eprintln!(
        "lspconf: a server is available at the pin, but `{cmd}` is implemented in ls01 \
         (spawn, framing drive, replay). This repo does not fake it."
    );
    ExitCode::from(EXIT_HARNESS_ERROR as u8)
}

// ------------------------------------------------------------- verify --

/// The server-free half: everything that can be wrong about a transcript long
/// before anything replays it.
fn verify(root: &Path, paths: &[&str]) -> ExitCode {
    let mut files: Vec<PathBuf> = Vec::new();
    if paths.is_empty() {
        collect_jsonl(&root.join("transcripts"), &mut files);
    } else {
        for p in paths {
            let p = PathBuf::from(p);
            if p.is_dir() {
                collect_jsonl(&p, &mut files);
            } else {
                files.push(p);
            }
        }
    }
    // Directory order is platform noise; sort before anything can depend on it.
    files.sort();

    let mut failures = 0u32;
    for path in &files {
        let display = lsp_harness::slash_path(path);
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{display}: unreadable: {e}");
                failures += 1;
                continue;
            }
        };
        let transcript = match jsonl::parse(&text) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{display}: {e}");
                failures += 1;
                continue;
            }
        };
        if let Err(errs) = transcript.validate() {
            for e in errs {
                eprintln!("{display}: {e}");
            }
            failures += 1;
            continue;
        }
        if jsonl::to_string(&transcript) != text {
            eprintln!(
                "{display}: not in canonical form (sorted keys, LF, trailing newline) \
                 — re-record rather than hand-editing"
            );
            failures += 1;
            continue;
        }
        println!("ok  {display} ({} records)", transcript.records.len());
    }

    let mut profiles: Vec<PathBuf> = Vec::new();
    collect_ext(&root.join("profiles"), "json", &mut profiles);
    profiles.sort();
    for path in &profiles {
        let display = lsp_harness::slash_path(path);
        match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).map_err(|e| e.to_string()))
        {
            Ok(v) if v.is_object() => println!("ok  {display}"),
            Ok(_) => {
                eprintln!("{display}: a capability profile must be a JSON object");
                failures += 1;
            }
            Err(e) => {
                eprintln!("{display}: {e}");
                failures += 1;
            }
        }
    }

    if files.is_empty() && profiles.is_empty() {
        println!(
            "verify: nothing to check yet — transcripts land in ls01, \
             which is gated on wolf-lang's s52 shipping `wolf lsp`"
        );
    }

    if failures == 0 {
        ExitCode::from(EXIT_OK as u8)
    } else {
        eprintln!("verify: {failures} file(s) failed");
        ExitCode::from(EXIT_MISMATCH as u8)
    }
}

fn collect_jsonl(dir: &Path, out: &mut Vec<PathBuf>) {
    collect_ext(dir, "jsonl", out);
}

fn collect_ext(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    // Sorted at every level: `read_dir` order is platform noise and must never
    // influence output a human reads or a machine diffs.
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_ext(&path, ext, out);
        } else if path.extension().is_some_and(|e| e == ext) {
            out.push(path);
        }
    }
}
