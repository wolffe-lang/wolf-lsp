//! `lspconf` — verify | doctor | record | replay | profiles | onetruth |
//! bench | fuzz.
//!
//! The harness front end. Every server-dependent path asks
//! [`lsp_harness::Doctor`] first and, when the answer is "no server at the
//! pin", prints
//!
//! ```text
//! SKIP: no wolf binary at pin 67c977f (…)
//! ```
//!
//! and exits `77`. `--require-server` turns that skip into a failure, which is
//! how a CI job that is *supposed* to have a binary proves it had one. Silence
//! is what turns a dark lane into a lane nobody notices is dark.
//!
//! As of the s52 pin bump the server exists and `READY` is reachable, so the
//! skip now means something narrower than it used to: not "the subcommand has
//! not been written", but "this machine has no binary at the pin". In CI that
//! is still every run — wolf-lang tags no releases, so there is no artifact to
//! download and this repo does not build the compiler (`vendor/README.md`).
//! Locally it takes one `cargo build` and a `WOLF_BIN`; see the README.
//!
//! Exit codes (mirroring the compiler and interpreter tracks, plus 77):
//! `0` matched · `1` mismatch / required server missing · `2` harness error ·
//! `77` skipped.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lsp_harness::profiles::{self, Provenance};
use lsp_harness::{
    Doctor, EXIT_HARNESS_ERROR, EXIT_MISMATCH, EXIT_OK, EXIT_SKIPPED, bench, fuzz, onetruth,
    record, replay,
};
use lsp_transcript::jsonl;

const USAGE: &str = "\
usage: lspconf [--require-server] <command> [args]

  server-free (this half always runs, on every platform):
    verify [path…]       parse, validate, and canonicalize transcripts
    profiles             validate every capability profile, and name the ones
                         no client has been read for yet
    doctor               report the pin, the binary that won resolution, and
                         whether a server is available

  server-dependent (skip with 77 when no binary resolves at the pin):
    record <script.lsps> drive a scripted session, write <script>.jsonl
    rerecord [dir]       re-record every script beside its transcript
    replay [path…]       drive recorded sessions and match per record
    onetruth [sample…]   assert publishDiagnostics == conform-run, per sample
    bench [--out F]      latency budgets, D5 JSONL (report-only)
    fuzz [--seed N]      seeded partial-edit session with three oracles

  options:
    --require-server     treat an unavailable server as a failure, not a skip
    --timings            `record` only: write the t_us sidecar
    --seed N             `fuzz` only: PRNG seed (default 1)
    --splices N          `fuzz` only: edits per session (default 64)
    --runs N             `bench` only: paired runs (default 10, D36's floor)
    --out PATH           `bench` only: JSONL destination (default stdout)
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
        Some("profiles") => profiles_cmd(&root),
        Some("doctor") => doctor(&root, require_server),
        Some(cmd @ ("record" | "rerecord" | "replay" | "onetruth" | "bench" | "fuzz")) => {
            server_command(&root, cmd, &positional[1..], &args, require_server)
        }
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

fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let prefix = format!("--{name}=");
    args.iter()
        .find_map(|a| a.strip_prefix(&prefix))
        .or_else(|| {
            let i = args.iter().position(|a| a == &format!("--{name}"))?;
            args.get(i + 1).map(String::as_str)
        })
}

fn flag_num<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    flag_value(args, name)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
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

/// Resolve the server, or print the skip and stop.
fn server_command(
    root: &Path,
    cmd: &str,
    rest: &[&str],
    raw: &[String],
    require_server: bool,
) -> ExitCode {
    let doc = Doctor::run(root);
    let Some(pin) = doc.pin.clone() else {
        return fail(&doc.availability.reason(None));
    };
    let bin = match &doc.availability {
        lsp_harness::Availability::Ready { path } => path.clone(),
        _ => {
            println!("{}", doc.skip_line().unwrap_or_default());
            println!("SKIP: `lspconf {cmd}` needs a server at the pin");
            return if doc.availability.is_skip() && !require_server {
                ExitCode::from(EXIT_SKIPPED as u8)
            } else {
                ExitCode::from(EXIT_MISMATCH as u8)
            };
        }
    };

    match cmd {
        "record" => record_cmd(root, &bin, &pin.commit, rest, raw),
        "rerecord" => rerecord_cmd(root, &bin, &pin.commit, rest, raw),
        "replay" => replay_cmd(root, &bin, &pin.commit, rest),
        "onetruth" => onetruth_cmd(root, &bin, rest),
        "bench" => bench_cmd(root, &bin, &pin.commit, rest, raw),
        "fuzz" => fuzz_cmd(root, &bin, rest, raw),
        _ => unreachable!("dispatched from a fixed list"),
    }
}

/// Today's date, `YYYY-MM-DD`, from the wall clock with no dependency.
///
/// Civil-from-days (Howard Hinnant's algorithm), because pulling a date crate
/// into a repo whose whole posture is dependency thinness, to stamp one header
/// field, is not a trade worth making.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn record_options(raw: &[String]) -> record::Options {
    record::Options {
        timings: raw.iter().any(|a| a == "--timings"),
    }
}

fn record_cmd(root: &Path, bin: &Path, pin: &str, rest: &[&str], raw: &[String]) -> ExitCode {
    let Some(script) = rest.first() else {
        return fail("record: name a script (`lspconf record transcripts/x/y.lsps`)");
    };
    let path = PathBuf::from(script);
    match record_one(root, bin, pin, &path, record_options(raw)) {
        Ok(out) => {
            println!("recorded {}", lsp_harness::slash_path(&out));
            ExitCode::from(EXIT_OK as u8)
        }
        Err(e) => fail(&e),
    }
}

fn record_one(
    root: &Path,
    bin: &Path,
    pin: &str,
    script: &Path,
    options: record::Options,
) -> Result<PathBuf, String> {
    let transcript = record::record(root, bin, script, pin, &today(), options)
        .map_err(|e| format!("{}: {e}", lsp_harness::slash_path(script)))?;
    let out = script.with_extension("jsonl");
    record::write(&transcript, &out).map_err(|e| e.to_string())?;
    Ok(out)
}

fn rerecord_cmd(root: &Path, bin: &Path, pin: &str, rest: &[&str], raw: &[String]) -> ExitCode {
    let dir = rest
        .first()
        .map_or_else(|| root.join("transcripts"), PathBuf::from);
    let mut scripts = Vec::new();
    collect_ext(&dir, "lsps", &mut scripts);
    scripts.sort();
    if scripts.is_empty() {
        return fail(&format!(
            "rerecord: no `.lsps` scripts under {}",
            lsp_harness::slash_path(&dir)
        ));
    }
    let options = record_options(raw);
    let mut failures = 0;
    for script in &scripts {
        match record_one(root, bin, pin, script, options) {
            Ok(out) => println!("recorded {}", lsp_harness::slash_path(&out)),
            Err(e) => {
                eprintln!("lspconf: {e}");
                failures += 1;
            }
        }
    }
    if failures == 0 {
        ExitCode::from(EXIT_OK as u8)
    } else {
        eprintln!("rerecord: {failures} script(s) failed");
        ExitCode::from(EXIT_MISMATCH as u8)
    }
}

fn replay_cmd(root: &Path, bin: &Path, pin: &str, rest: &[&str]) -> ExitCode {
    let mut files = Vec::new();
    if rest.is_empty() {
        collect_ext(&root.join("transcripts"), "jsonl", &mut files);
    } else {
        for p in rest {
            let p = PathBuf::from(p);
            if p.is_dir() {
                collect_ext(&p, "jsonl", &mut files);
            } else {
                files.push(p);
            }
        }
    }
    files.sort();
    if files.is_empty() {
        println!("replay: no transcripts to replay");
        return ExitCode::from(EXIT_OK as u8);
    }

    let mut mismatched = 0u32;
    let mut errored = 0u32;
    for file in &files {
        match replay::replay(root, bin, file, pin) {
            Ok(report) if report.ok() => {
                println!(
                    "ok  {} — {} record(s) matched",
                    report.name, report.compared
                );
            }
            Ok(report) => {
                println!(
                    "FAIL {} ({}) — {} of {} record(s) did not match",
                    report.name,
                    report.file,
                    report.mismatches.len(),
                    report.compared
                );
                for f in &report.mismatches {
                    print!("{f}");
                }
                mismatched += 1;
            }
            Err(e) => {
                eprintln!("ERROR {}: {e}", lsp_harness::slash_path(file));
                errored += 1;
            }
        }
    }
    if errored > 0 {
        ExitCode::from(EXIT_HARNESS_ERROR as u8)
    } else if mismatched > 0 {
        ExitCode::from(EXIT_MISMATCH as u8)
    } else {
        ExitCode::from(EXIT_OK as u8)
    }
}

fn onetruth_cmd(root: &Path, bin: &Path, rest: &[&str]) -> ExitCode {
    let workspace = root.join("vendor").join("upstream").join("samples");
    let samples: Vec<String> = if rest.is_empty() {
        match lsp_harness::samples(root) {
            Ok(s) => s,
            Err(e) => return fail(&e),
        }
    } else {
        rest.iter().map(|s| (*s).to_string()).collect()
    };
    let (loaded, errors) = profiles::load_all(root);
    for e in &errors {
        eprintln!("lspconf: {e}");
    }
    if !errors.is_empty() {
        return ExitCode::from(EXIT_HARNESS_ERROR as u8);
    }
    let ledger = match onetruth::load_ledger(root) {
        Ok(l) => l,
        Err(e) => return fail(&format!("onetruth: {e}")),
    };

    // Under every encoding: the transform is under test here as much as the
    // agreement is, and a divergence that shows up only in utf-16 is exactly
    // the bug §5 exists for.
    let order = ["minimal", "utf8-first", "utf32-only"];
    let mut unknown = 0u32;
    let mut matched: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for name in order {
        let Some(profile) = loaded.get(name) else {
            return fail(&format!("onetruth: no profile `{name}`"));
        };
        for sample in &samples {
            let divergence = match onetruth::check(root, bin, &workspace, sample, profile) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("ERROR {sample}: {e}");
                    return ExitCode::from(EXIT_HARNESS_ERROR as u8);
                }
            };
            if divergence.is_empty() {
                println!("ok  {sample} ({})", divergence.encoding);
                continue;
            }
            let (known, new) = onetruth::triage(&divergence, &ledger);
            for (entry, what) in &known {
                matched.insert(entry.id.clone());
                println!(
                    "KNOWN {sample} ({}) {} [{}] — {what}",
                    divergence.encoding, entry.id, entry.filed
                );
            }
            if !new.is_empty() {
                println!("DIVERGENCE {sample} ({})", divergence.encoding);
                print!("{}", divergence.filing());
                unknown += 1;
            }
        }
    }

    // A ledger entry that matched nothing is a bug that was fixed and a note
    // that outlived it — which is how the same bug gets re-introduced without
    // anyone noticing. Failing here is the point.
    let stale: Vec<&onetruth::Filed> = ledger.iter().filter(|f| !matched.contains(&f.id)).collect();
    for entry in &stale {
        eprintln!(
            "STALE {} — `divergences.toml` records a divergence on {} ({}) that no longer \
             happens. Delete the entry (and close the upstream issue).",
            entry.id, entry.sample, entry.code
        );
    }

    let pending: Vec<&onetruth::Filed> = ledger
        .iter()
        .filter(|f| matched.contains(&f.id) && f.filed == "pending")
        .collect();
    for entry in &pending {
        println!(
            "TODO  {} is `filed = \"pending\"` — open the upstream issue and record the link",
            entry.id
        );
    }

    if unknown == 0 && stale.is_empty() {
        println!(
            "onetruth: {} sample(s) x {} encoding(s); {} known divergence(s), zero unfiled",
            samples.len(),
            order.len(),
            matched.len()
        );
        ExitCode::from(EXIT_OK as u8)
    } else {
        eprintln!(
            "onetruth: {unknown} unfiled divergence(s), {} stale ledger entr(ies). \
             A divergence is a wolf-lang bug: file it upstream with both records and add \
             it to `divergences.toml` — never normalize it away here.",
            stale.len()
        );
        ExitCode::from(EXIT_MISMATCH as u8)
    }
}

fn bench_cmd(root: &Path, bin: &Path, pin: &str, rest: &[&str], raw: &[String]) -> ExitCode {
    let workspace = root.join("vendor").join("upstream").join("samples");
    let sample = rest.first().copied().unwrap_or("regions.lu");
    let runs = flag_num(raw, "runs", bench::MIN_RUNS);
    let (loaded, _) = profiles::load_all(root);
    let Some(profile) = loaded.get("minimal") else {
        return fail("bench: no profile `minimal`");
    };
    let run = match bench::measure(bin, &workspace, sample, profile, runs) {
        Ok(r) => r,
        Err(e) => return fail(&format!("bench: {e}")),
    };
    let config = serde_json::json!({
        "wolf_pin": pin,
        "sample": sample,
        "sample_tier": "corpus",
        "profile": profile.profile,
        "encoding": profile.expects_encoding.as_str(),
        "resident": false,
        "report_only": true,
    });
    let jsonl = bench::to_jsonl(&run, pin, &config);
    if run.under_sampled {
        eprintln!(
            "bench: {runs} run(s) is below D36's floor of {} — the medians are emitted and \
             the record says so, but do not compare them against anything",
            bench::MIN_RUNS
        );
    }
    match flag_value(raw, "out") {
        Some(path) => {
            let path = PathBuf::from(path);
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&path, &jsonl) {
                return fail(&format!("bench: {}: {e}", lsp_harness::slash_path(&path)));
            }
            println!("wrote {}", lsp_harness::slash_path(&path));
        }
        None => print!("{jsonl}"),
    }
    for series in &run.series {
        eprintln!(
            "  {:<34} {:>8.1} ms p50  {:>8.1} ms p95  ±{:.1} MAD  [{}]",
            series.bench,
            series.p50(),
            series.p95(),
            series.mad(),
            series.class
        );
    }
    ExitCode::from(EXIT_OK as u8)
}

fn fuzz_cmd(root: &Path, bin: &Path, rest: &[&str], raw: &[String]) -> ExitCode {
    let workspace = root.join("vendor").join("upstream").join("samples");
    let sample = rest.first().copied().unwrap_or("regions.lu");
    let seed: u64 = flag_num(raw, "seed", 1);
    let splices: usize = flag_num(raw, "splices", 64);
    let (loaded, _) = profiles::load_all(root);
    let Some(profile) = loaded.get("minimal") else {
        return fail("fuzz: no profile `minimal`");
    };
    match fuzz::run(bin, &workspace, sample, profile, seed, splices) {
        Ok(outcome) if outcome.ok() => {
            println!("ok  fuzz {sample} seed={seed} splices={splices} — three oracles held");
            ExitCode::from(EXIT_OK as u8)
        }
        Ok(outcome) => {
            println!("FAIL fuzz {sample} seed={seed} splices={splices}");
            for f in &outcome.failures {
                println!("  {f}");
            }
            println!("  reproduce: lspconf fuzz {sample} --seed {seed} --splices {splices}");
            ExitCode::from(EXIT_MISMATCH as u8)
        }
        Err(e) => fail(&format!("fuzz: {e}")),
    }
}

// ----------------------------------------------------------- profiles --

fn profiles_cmd(root: &Path) -> ExitCode {
    let (loaded, errors) = profiles::load_all(root);
    for e in &errors {
        eprintln!("{e}");
    }
    for (name, profile) in &loaded {
        let kind = match &profile.provenance {
            Provenance::Synthetic { .. } => "synthetic".to_string(),
            Provenance::Derived { client, commit, .. } => {
                format!("derived from {client}@{}", &commit[..7.min(commit.len())])
            }
        };
        println!(
            "ok  profiles/{name}.json — {kind}, expects {}",
            profile.expects_encoding
        );
    }

    // The honest half. Six real clients are named by the sprint; a run that
    // printed only the profiles that exist would report "all validated" while
    // the clients that matter had never been read.
    let missing = profiles::missing_derived(&loaded);
    if missing.is_empty() {
        println!("profiles: every named client has a derived profile");
    } else {
        println!(
            "\nNOT YET DERIVED — {} of {} real clients have no profile read off the client:",
            missing.len(),
            profiles::REAL_CLIENTS.len()
        );
        for (client, sprint) in &missing {
            println!("  {client:<12} owed by {sprint}");
        }
        println!(
            "A profile invented here rather than read off a client is a fiction the suite \
             would then test against, so these stay absent until their sprint reads them."
        );
    }

    if errors.is_empty() {
        ExitCode::from(EXIT_OK as u8)
    } else {
        eprintln!("profiles: {} document(s) failed", errors.len());
        ExitCode::from(EXIT_MISMATCH as u8)
    }
}

// ------------------------------------------------------------- verify --

/// The server-free half: everything that can be wrong about a transcript long
/// before anything replays it.
fn verify(root: &Path, paths: &[&str]) -> ExitCode {
    let mut files: Vec<PathBuf> = Vec::new();
    if paths.is_empty() {
        collect_ext(&root.join("transcripts"), "jsonl", &mut files);
    } else {
        for p in paths {
            let p = PathBuf::from(p);
            if p.is_dir() {
                collect_ext(&p, "jsonl", &mut files);
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
        // A transcript whose script is gone cannot be re-recorded, which makes
        // it a golden byte file by accident — the artifact this design exists
        // to not be.
        let script = path.with_extension("lsps");
        if !script.is_file() {
            eprintln!(
                "{display}: no `{}` beside it — a transcript with no script cannot be \
                 re-recorded, and an un-re-recordable transcript is a golden byte file",
                script
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
            );
            failures += 1;
            continue;
        }
        println!("ok  {display} ({} records)", transcript.records.len());
    }

    // Scripts parse, and name a profile that exists.
    let mut scripts: Vec<PathBuf> = Vec::new();
    collect_ext(&root.join("transcripts"), "lsps", &mut scripts);
    scripts.sort();
    for path in &scripts {
        let display = lsp_harness::slash_path(path);
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{display}: unreadable: {e}");
                failures += 1;
                continue;
            }
        };
        match lsp_harness::Script::parse(&text) {
            Ok(script) => {
                let profile = root
                    .join("profiles")
                    .join(format!("{}.json", script.profile));
                if !profile.is_file() {
                    eprintln!(
                        "{display}: names profile `{}`, which does not exist",
                        script.profile
                    );
                    failures += 1;
                    continue;
                }
                if !root.join(&script.workspace).is_dir() {
                    eprintln!("{display}: workspace `{}` does not exist", script.workspace);
                    failures += 1;
                    continue;
                }
                println!("ok  {display} ({} steps)", script.steps.len());
            }
            Err(e) => {
                eprintln!("{display}: {e}");
                failures += 1;
            }
        }
    }

    if files.is_empty() && scripts.is_empty() {
        println!("verify: nothing to check — no transcripts and no scripts");
    }

    if failures == 0 {
        ExitCode::from(EXIT_OK as u8)
    } else {
        eprintln!("verify: {failures} file(s) failed");
        ExitCode::from(EXIT_MISMATCH as u8)
    }
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
