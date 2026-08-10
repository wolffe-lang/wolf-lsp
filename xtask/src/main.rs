//! Repo automation (`cargo xtask <command>`). CI-shaped: pure, exit-code
//! driven — the workflows are thin wrappers over these commands.
//!
//! `xtask` deliberately depends on **no workspace crate** (only `serde_json`),
//! mirroring wolf-lang. That is not tidiness: `sync-pin` and `independence`
//! must be runnable when the harness itself does not compile, which is exactly
//! the moment someone is most likely to reach for them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("ci") => ci(),
        Some("sync-pin") => sync_pin(),
        Some("vendor-check") => vendor_check(),
        Some("independence") => independence(),
        Some("fixtures-check") => fixtures_check(),
        _ => {
            eprintln!("usage: cargo xtask <ci|sync-pin|vendor-check|independence|fixtures-check>");
            ExitCode::from(2)
        }
    }
}

fn repo_root() -> PathBuf {
    // The manifest dir is `<root>/xtask`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent")
        .to_path_buf()
}

// ------------------------------------------------------------------- ci --

/// fmt-check + clippy (deny warnings) + tests + pin integrity + independence
/// + the server-free doctor report.
fn ci() -> ExitCode {
    let steps: &[(&str, &[&str])] = &[
        ("fmt", &["fmt", "--all", "--check"]),
        (
            "clippy",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        ("test", &["test", "--workspace"]),
        ("sync-pin", &["xtask", "sync-pin"]),
        ("independence", &["xtask", "independence"]),
        ("fixtures-check", &["xtask", "fixtures-check"]),
    ];
    for (name, args) in steps {
        eprintln!("== xtask ci: {name}");
        let ok = Command::new("cargo")
            .current_dir(repo_root())
            .args(*args)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            eprintln!("xtask ci: step `{name}` failed");
            return ExitCode::FAILURE;
        }
    }

    // `doctor` is the one step whose *success* includes exit 77: no server at
    // the pin is the expected state until wolf-lang's s52 ships `wolf lsp`.
    // Treating 77 as a failure here would make the whole gate red for a
    // condition the sprint explicitly designs for.
    eprintln!("== xtask ci: doctor");
    let status = Command::new("cargo")
        .current_dir(repo_root())
        .args(["run", "--quiet", "--bin", "lspconf", "--", "doctor"])
        .status();
    match status.map(|s| s.code()) {
        Ok(Some(0)) => eprintln!("xtask ci: doctor reports a server at the pin"),
        Ok(Some(77)) => eprintln!("xtask ci: doctor reports SERVER UNAVAILABLE (expected pre-s52)"),
        other => {
            eprintln!("xtask ci: doctor failed ({other:?})");
            return ExitCode::FAILURE;
        }
    }

    eprintln!("xtask ci: all steps green");
    ExitCode::SUCCESS
}

// ------------------------------------------------------------- sync-pin --

/// Snapshot integrity, plus the submodule cross-check when it is initialized.
///
/// CI only ever runs the first half: the upstream repo is private and org
/// policy disables deploy keys, so CI cannot clone the submodule at all
/// (`vendor/README.md`). Locally, both halves run.
fn sync_pin() -> ExitCode {
    let root = repo_root();
    let mut errors = vendor_errors(&root);

    let submodule = root.join("upstream");
    if submodule.join("spec").is_dir() {
        eprintln!("sync-pin: submodule present — cross-checking the snapshot");
        errors.extend(submodule_errors(&root, &submodule));
    } else {
        eprintln!(
            "sync-pin: submodule absent — snapshot not cross-checked here \
             (expected in CI; run `git submodule update --init upstream` locally)"
        );
    }

    report("sync-pin", errors)
}

/// The snapshot-integrity half on its own.
fn vendor_check() -> ExitCode {
    report("vendor-check", vendor_errors(&repo_root()))
}

fn report(cmd: &str, errors: Vec<String>) -> ExitCode {
    if errors.is_empty() {
        eprintln!("{cmd}: ok");
        return ExitCode::SUCCESS;
    }
    for e in &errors {
        eprintln!("{cmd}: {e}");
    }
    eprintln!("{cmd}: {} problem(s)", errors.len());
    ExitCode::FAILURE
}

/// Everything checkable without the submodule: the PIN is well-formed, the
/// grammar is there, and `samples.toml` and `samples/` agree exactly.
fn vendor_errors(root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let vendor = root.join("vendor").join("upstream");

    errors.extend(pin_errors(&vendor.join("PIN")));

    if !vendor.join("spec").join("grammar.ebnf").is_file() {
        errors.push(
            "vendor/upstream/spec/grammar.ebnf is missing (ls05's drift check reads it)"
                .to_string(),
        );
    }

    let manifest = vendor.join("samples.toml");
    let listed = match std::fs::read_to_string(&manifest) {
        Ok(text) => sample_paths(&text),
        Err(e) => {
            errors.push(format!("vendor/upstream/samples.toml: {e}"));
            return errors;
        }
    };
    if listed.is_empty() {
        errors.push("samples.toml lists no samples".to_string());
    }

    let samples_dir = vendor.join("samples");
    let mut present: Vec<PathBuf> = Vec::new();
    walk(&samples_dir, &mut present);
    let present: BTreeSet<String> = present
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "lu"))
        .map(|p| slash(p.strip_prefix(&samples_dir).unwrap_or(p)))
        .collect();

    for path in &listed {
        if !present.contains(path) {
            errors.push(format!(
                "samples.toml lists `{path}`, which is not in vendor/upstream/samples/"
            ));
        }
    }
    for path in &present {
        if !listed.contains(path) {
            errors.push(format!(
                "vendor/upstream/samples/{path} is not listed in samples.toml \
                 — no ad-hoc copies: every sample carries the editor reason it was picked (ls00 §5)"
            ));
        }
    }
    errors
}

/// Structural PIN validation. The full parse lives in `lsp_harness::pin`; this
/// is the copy that has to work when the harness does not compile.
fn pin_errors(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return vec![format!(
            "{} is missing — this repo does not know which wolf-lang commit it targets",
            slash(path)
        )];
    };
    let mut errors = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            errors.push(format!("PIN: not `key = value`: {raw:?}"));
            continue;
        };
        let (key, value) = (key.trim(), value.trim().trim_matches('"'));
        seen.insert(match key {
            "commit" => {
                if value.len() != 40 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
                    errors.push(format!(
                        "PIN: `commit` must be a full 40-char sha, got {value:?} \
                         — an abbreviated sha is a moving target as upstream grows"
                    ));
                }
                "commit"
            }
            "version" => {
                if value.is_empty() {
                    errors
                        .push("PIN: `version` is empty; doctor has nothing to compare".to_string());
                }
                "version"
            }
            "serves_lsp" => {
                if value != "true" && value != "false" {
                    errors.push(format!(
                        "PIN: `serves_lsp` must be true or false, got {value:?}"
                    ));
                }
                "serves_lsp"
            }
            other => {
                errors.push(format!("PIN: unknown key {other:?}"));
                ""
            }
        });
    }
    for required in ["commit", "version", "serves_lsp"] {
        if !seen.contains(required) {
            errors.push(format!("PIN: no `{required}`"));
        }
    }
    errors
}

/// Byte-compare the snapshot against the submodule, **in both directions**.
///
/// wolf-interp's equivalent proves only submodule ⊆ vendor, so a stale extra
/// file in `vendor/` survives a re-vendor unnoticed. Drift is drift either way.
fn submodule_errors(root: &Path, submodule: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let vendor = root.join("vendor").join("upstream");

    let mut pairs: Vec<(PathBuf, PathBuf)> = vec![(
        submodule.join("spec").join("grammar.ebnf"),
        vendor.join("spec").join("grammar.ebnf"),
    )];
    let listed = std::fs::read_to_string(vendor.join("samples.toml"))
        .map(|t| sample_paths(&t))
        .unwrap_or_default();
    for rel in &listed {
        pairs.push((
            submodule.join("corpus").join(rel),
            vendor.join("samples").join(rel),
        ));
    }

    for (src, dst) in pairs {
        match (std::fs::read(&src), std::fs::read(&dst)) {
            (Ok(a), Ok(b)) if a == b => {}
            (Ok(_), Ok(_)) => errors.push(format!(
                "vendor drift: {} differs from the submodule at the pin — re-vendor, never hand-edit",
                slash(&dst)
            )),
            (Err(e), _) => errors.push(format!("{}: {e} (is the submodule at the pin?)", slash(&src))),
            (_, Err(e)) => errors.push(format!("{}: {e}", slash(&dst))),
        }
    }
    errors
}

fn sample_paths(manifest: &str) -> BTreeSet<String> {
    // A four-key manifest does not justify a TOML dependency (ls00 §7 keeps
    // this repo's dependencies thin); `path = "…"` inside `[[sample]]` is the
    // whole grammar, and `vendor-check` fails loudly if it drifts from reality.
    let mut out = BTreeSet::new();
    let mut in_sample = false;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_sample = line == "[[sample]]";
            continue;
        }
        if !in_sample {
            continue;
        }
        if let Some(rest) = line.strip_prefix("path")
            && let Some((_, value)) = rest.split_once('=')
        {
            out.insert(value.trim().trim_matches('"').to_string());
        }
    }
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    // Sorted at every level: directory order is platform noise.
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Render a path with `/` separators on every platform.
///
/// Naively joining components with `/` doubles the leading separator on unix
/// (the root component is itself `/`), which is how a report ends up saying
/// `//home/...`. Mirrors `lsp_harness::slash_path`.
fn slash(path: &Path) -> String {
    let mut out = String::new();
    for component in path.components() {
        let text = component.as_os_str().to_string_lossy();
        if text == "/" || text == "\\" {
            out.push('/');
            continue;
        }
        if !out.is_empty() && !out.ends_with('/') {
            out.push('/');
        }
        out.push_str(&text);
    }
    out
}

// ------------------------------------------------------ fixtures-check --

/// **A local `.lu` exists only where a recorded gap says it may.**
///
/// ls00 §5's rule is that fixtures are requested upstream, never forked
/// locally: a file authored here is not canonical `wolf fmt` output, does not
/// participate in the compiler's own corpus runs, and drifts silently from the
/// language it claims to be written in. ls01 §5 forced one exception —
/// astral-plane text, which no corpus file at the pin contains — and an
/// exception with no fence around it is just the rule being abandoned.
///
/// So: every `.lu` under `fixtures/` must be claimed by a `local_stopgap` in a
/// `[gap.*]` table of `samples.toml`, every `local_stopgap` must exist, and
/// each fixture must name its gap in its own text so the file explains itself
/// to whoever finds it in three sprints. A second fixture appearing without a
/// gap entry fails this check, which is the whole point: the next person who
/// wants to skip the upstream request has to write down why.
fn fixtures_check() -> ExitCode {
    let root = repo_root();
    let mut errors = Vec::new();
    let fixtures = root.join("fixtures");
    let manifest = root.join("vendor").join("upstream").join("samples.toml");

    let text = match std::fs::read_to_string(&manifest) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("fixtures-check: {}: {e}", slash(&manifest));
            return ExitCode::FAILURE;
        }
    };
    // `local_stopgap = "fixtures/x.lu"` inside a `[gap.*]` table, plus the gap
    // name it belongs to. Same hand-rolled reader as `sample_paths`, and the
    // same reason: a four-key manifest does not justify a TOML dependency.
    let mut claimed: BTreeSet<String> = BTreeSet::new();
    let mut gap = String::new();
    let mut gaps: Vec<(String, String)> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("[gap.") {
            gap = rest.trim_end_matches(']').to_string();
            continue;
        }
        if line.starts_with('[') {
            gap.clear();
            continue;
        }
        if gap.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("local_stopgap")
            && let Some((_, value)) = rest.split_once('=')
        {
            let path = value.trim().trim_matches('"').to_string();
            gaps.push((gap.clone(), path.clone()));
            claimed.insert(path);
        }
    }

    for (gap, path) in &gaps {
        let full = root.join(path);
        if !full.is_file() {
            errors.push(format!(
                "[gap.{gap}] names `{path}` as its local_stopgap, but no such file exists \
                 — a gap whose workaround is gone should be closed or re-opened, not left \
                 pointing at nothing"
            ));
            continue;
        }
        let body = std::fs::read_to_string(&full).unwrap_or_default();
        if !body.contains(&format!("gap.{gap}")) {
            errors.push(format!(
                "{path} does not name `[gap.{gap}]` in its own text — a fixture that does \
                 not explain why it is allowed to exist is a fork with a comment"
            ));
        }
    }

    let mut present: Vec<PathBuf> = Vec::new();
    walk(&fixtures, &mut present);
    for path in present
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "lu"))
    {
        let rel = slash(path.strip_prefix(&root).unwrap_or(path));
        if !claimed.contains(&rel) {
            errors.push(format!(
                "{rel} is a locally-authored `.lu` that no `[gap.*]` entry claims. \
                 Fixtures are requested upstream, never forked locally (ls00 §5); the ONE \
                 exception is recorded in samples.toml with a `local_stopgap` and a \
                 `retire_when`. Add the gap entry, or use a corpus sample."
            ));
        }
    }

    report("fixtures-check", errors)
}

// -------------------------------------------------------- independence --

/// **The editor layer talks to a process, or it does not talk** (D34).
///
/// Four laws, each catching something the others cannot:
/// 1. No cargo dependency on a `wolf_*` crate, anywhere in the graph.
/// 2. No `wolf-lang` git dependency in the lock file.
/// 3. No source file imports a compiler-side crate (catches a path or vendored
///    dependency the lock file would not show).
/// 4. Nothing reaches into `upstream/` outside `spec/` and `corpus/` — this
///    repo consumes *data* from the pin, never source, and never builds it.
fn independence() -> ExitCode {
    let root = repo_root();
    let mut violations: Vec<String> = Vec::new();

    // 1 + 2: the dependency graph.
    match Command::new("cargo")
        .current_dir(&root)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
    {
        Ok(out) if out.status.success() => {
            let meta: serde_json::Value =
                serde_json::from_slice(&out.stdout).expect("cargo metadata: invalid json");
            for pkg in meta["packages"].as_array().into_iter().flatten() {
                let name = pkg["name"].as_str().unwrap_or("<unnamed>");
                for dep in pkg["dependencies"].as_array().into_iter().flatten() {
                    let dep_name = dep["name"].as_str().unwrap_or("");
                    if dep_name.starts_with("wolf_") || dep_name == "wolf-lang" {
                        violations.push(format!(
                            "ILLEGAL DEPENDENCY {name} -> {dep_name}: this repo talks to a `wolf` \
                             process over stdio, never links against the compiler (D34)"
                        ));
                    }
                }
            }
        }
        other => violations.push(format!("cargo metadata failed: {other:?}")),
    }

    if let Ok(lock) = std::fs::read_to_string(root.join("Cargo.lock")) {
        for line in lock.lines() {
            let line = line.trim();
            if line.starts_with("name = \"wolf_") {
                violations.push(format!("Cargo.lock names a compiler crate: {line}"));
            }
            if line.contains("wolf-lang") {
                violations.push(format!("Cargo.lock has a wolf-lang dependency: {line}"));
            }
        }
    }

    // 3 + 4: the source tree. Scanned in Rust rather than with `grep -P`, which
    // is GNU-only and would make this gate linux-only.
    //
    // The forbidden prefixes are ASSEMBLED rather than written out, so this
    // file does not match its own rule. A gate whose first finding is itself
    // gets an exemption bolted on, and the exemption is the hole.
    let forbidden: Vec<String> = ["crates", "xtask", "src", "Cargo"]
        .iter()
        .map(|s| format!("upstream/{s}"))
        .collect();
    let mut sources: Vec<PathBuf> = Vec::new();
    for dir in ["src", "crates", "xtask", ".github"] {
        walk_skipping(&root.join(dir), &mut sources);
    }
    sources.sort();
    for path in &sources {
        let interesting = path
            .extension()
            .is_some_and(|e| e == "rs" || e == "toml" || e == "yml" || e == "yaml");
        if !interesting {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            // Prose naming a compiler crate is fine and often necessary; an
            // *import* is the doctrine breaking.
            let imports = code.starts_with("use wolf_")
                || code.starts_with("pub use wolf_")
                || code.starts_with("extern crate wolf_");
            if imports {
                violations.push(format!(
                    "{}:{}: imports a compiler crate: {code}",
                    slash(path),
                    n + 1
                ));
            }
            for reach in &forbidden {
                if code.contains(reach.as_str()) {
                    violations.push(format!(
                        "{}:{}: reaches into {reach} — only spec/ and corpus/ are consumed from the pin",
                        slash(path),
                        n + 1
                    ));
                }
            }
        }
    }

    if violations.is_empty() {
        eprintln!("independence: ok — no compiler code, no compiler build, stdio only");
        ExitCode::SUCCESS
    } else {
        for v in &violations {
            eprintln!("independence: {v}");
        }
        eprintln!("independence: {} violation(s)", violations.len());
        ExitCode::FAILURE
    }
}

fn walk_skipping(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if matches!(name, "target" | "upstream" | ".git" | "node_modules") {
            continue;
        }
        if path.is_dir() {
            walk_skipping(&path, out);
        } else {
            out.push(path);
        }
    }
}
