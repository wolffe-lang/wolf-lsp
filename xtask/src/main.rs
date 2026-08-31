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

mod compat;
mod config;
mod release;
mod vscode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("ci") => ci(),
        Some("sync-pin") => sync_pin(),
        Some("vendor-check") => vendor_check(),
        Some("independence") => independence(),
        Some("fixtures-check") => fixtures_check(),
        Some("nvim-check") => nvim_derived(false),
        // ls06 §§1-3: the config tier's cross-editor invariants, and the emacs
        // keyword drift gate. `config-check` needs no editor installed; the
        // lanes that DO drive `hx` and `emacs` live in the workflow, because a
        // gate that silently passes when its editor is absent is not a gate.
        Some("config-check") => report("config-check", config::check(&repo_root())),
        Some("emacs-check") => report("emacs-check", config::emacs(&repo_root())),
        // The one lane that needs an editor installed. Exit 77 with a reason
        // when `hx` is absent, matching `doctor` — a dark lane that says
        // nothing is a lane nobody notices is dark (ls00 §3).
        Some("helix-health") => match config::helix_health(&repo_root()) {
            config::Health::Ok(errors) => report("helix-health", errors),
            config::Health::Skip(reason) => {
                println!("SKIP: helix-health — {reason}");
                ExitCode::from(77)
            }
        },
        Some("nvim-generate") => nvim_derived(true),
        // ls05 §2 names `grammar-drift`; `vscode-check` is the alias that
        // matches the `nvim-check`/`nvim-generate` family. Same command.
        Some("grammar-drift" | "vscode-check") => {
            report("grammar-drift", vscode::derived(&repo_root(), false))
        }
        Some("grammar-generate" | "vscode-generate") => {
            report("grammar-generate", vscode::derived(&repo_root(), true))
        }
        // ls07 §3: the compatibility statement. `compat-check` is a gate on
        // every push; `compat-generate` rewrites the two client artifacts and
        // the docs/COMPAT.md table from `clients/*/compat.json`.
        Some("compat-check") => report("compat-check", compat::check(&repo_root(), false)),
        Some("compat-generate") => report("compat-generate", compat::check(&repo_root(), true)),
        // ls07 §2: compute and VERIFY the wolf.nvim mirror split. Pushes
        // nothing — the mirror repository does not exist, and creating it is a
        // human act (docs/DISTRIBUTION.md).
        Some("nvim-split") => {
            let into = args
                .iter()
                .position(|a| a == "--into")
                .and_then(|i| args.get(i + 1));
            match release::nvim_split(&repo_root(), into.map(Path::new)) {
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("nvim-split: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        // ls07 §4: the nine-step release checklist, executed as far as this
        // repository can execute it. Steps needing a marketplace, a token, a
        // tagged wolf-lang release or a person at a clean machine report
        // PENDING and never silently vanish.
        Some("release-check") => {
            if release::check(&repo_root()).render() == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        _ => {
            eprintln!(
                "usage: cargo xtask \
                 <ci|sync-pin|vendor-check|independence|fixtures-check\
                 |nvim-check|nvim-generate|grammar-drift|grammar-generate\
                 |config-check|emacs-check|helix-health\
                 |compat-check|compat-generate|nvim-split|release-check>"
            );
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
/// + the derived-artifact checks + the server-free doctor report.
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
        ("nvim-check", &["xtask", "nvim-check"]),
        ("grammar-drift", &["xtask", "grammar-drift"]),
        ("config-check", &["xtask", "config-check"]),
        ("emacs-check", &["xtask", "emacs-check"]),
        // ls07 §3. Deliberately in the always-on gate rather than in a release
        // lane: the moment a `compat.json` range outruns its evidence is the
        // moment somebody edited it, not the moment somebody tagged.
        ("compat-check", &["xtask", "compat-check"]),
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

    // ls07 §4: the release checklist, run on every push rather than only at a
    // tag. A checklist first executed on release day is a checklist whose first
    // execution is a discovery, and its PENDING rows are the standing list of
    // what this repository still owes a human — a list that is worth reading
    // when it grows, not when somebody is trying to ship.
    //
    // Last, and deliberately re-running gates the loop above already ran: the
    // steps above exist so a failure names itself precisely, and a checklist
    // that trusts an earlier command's exit code instead of running the command
    // is not a checklist. Everything it re-runs is cached by then.
    eprintln!("== xtask ci: release-check");
    let failures = release::check(&repo_root()).render();
    if failures > 0 {
        eprintln!("xtask ci: release-check reported {failures} failure(s)");
        return ExitCode::FAILURE;
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
pub(crate) fn slash(path: &Path) -> String {
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

// ---------------------------------------------------------- nvim-check --

/// **Everything derived in `clients/nvim/` still agrees with the pin.**
///
/// The Neovim plugin carries two artifacts that are *read off* the pinned
/// toolchain rather than authored:
///
/// 1. `syntax/wolf.vim`'s keyword set, which must be exactly `reserved_kw`
///    from `vendor/upstream/spec/grammar.ebnf`. This is ls04 §2's drift check,
///    and the same discipline `clients/fackr/inventory.md` and
///    `clients/facsimile/inventory.md` record for their token tables. Three
///    independently-written tables from one pinned grammar cross-check the
///    extraction; a drift in one is a drift in all three.
/// 2. `lua/wolf/pin.lua`, generated from `vendor/upstream/PIN` so
///    `:checkhealth wolf` can compare a user's `wolf --version` against the
///    build this plugin was verified with. A hand-maintained copy of the pin
///    is a copy that goes stale on the first pin bump, silently, in the one
///    place whose whole job is noticing staleness.
///
/// `nvim-generate` writes (1) is never generated — the `.vim` file is written
/// by a human with the grammar open, and this check is what makes that safe.
///
/// **What this check deliberately does NOT cover: operators.** `grammar.ebnf`
/// is generated by `wolf xtask spec-extract` and the extraction elides the
/// precedence climb — the quoted terminals it carries are missing `==` `!=`
/// `<` `>` `<=` `>=` `<=>` `&&` `||` `<<` `>>` `/` `&`, which live only in
/// `spec/01-grammar.md` §3.2 and are not vendored. So an operator change
/// upstream is invisible here. That gap is recorded in every client's
/// `inventory.md`, is open upstream, and is stated rather than papered over
/// with a check that only appears to cover it.
fn nvim_derived(write: bool) -> ExitCode {
    let root = repo_root();
    let mut errors: Vec<String> = Vec::new();
    let nvim = root.join("clients").join("nvim");

    // --- (1) the keyword set -------------------------------------------
    let ebnf = root
        .join("vendor")
        .join("upstream")
        .join("spec")
        .join("grammar.ebnf");
    match std::fs::read_to_string(&ebnf) {
        Ok(text) => {
            let expected = reserved_keywords(&text);
            if expected.is_empty() {
                errors.push(format!(
                    "{}: no `reserved_kw ::=` rule found — the extraction this check \
                     depends on has changed shape",
                    slash(&ebnf)
                ));
            }
            let syntax = nvim.join("syntax").join("wolf.vim");
            match std::fs::read_to_string(&syntax) {
                Ok(vim) => match syntax_keywords(&vim) {
                    Some(found) => {
                        for missing in expected.difference(&found) {
                            errors.push(format!(
                                "syntax/wolf.vim is missing the reserved keyword `{missing}` \
                                 — a keyword the compiler knows and the editor does not \
                                 renders as an identifier"
                            ));
                        }
                        for extra in found.difference(&expected) {
                            errors.push(format!(
                                "syntax/wolf.vim colours `{extra}` as a reserved keyword, and \
                                 `reserved_kw` does not contain it — an invented keyword \
                                 teaches a language that does not exist"
                            ));
                        }
                        if errors.is_empty() {
                            eprintln!(
                                "nvim: syntax/wolf.vim carries all {} reserved keywords, and no others",
                                expected.len()
                            );
                        }
                    }
                    None => errors.push(
                        "syntax/wolf.vim has no `reserved-kw-begin`/`reserved-kw-end` markers \
                         — the drift check reads the keyword set from between them"
                            .to_string(),
                    ),
                },
                Err(e) => errors.push(format!("{}: {e}", slash(&syntax))),
            }
        }
        Err(e) => errors.push(format!("{}: {e}", slash(&ebnf))),
    }

    // --- (2) the generated pin constant --------------------------------
    let pin_path = root.join("vendor").join("upstream").join("PIN");
    match std::fs::read_to_string(&pin_path) {
        Ok(text) => {
            let want = pin_lua(&text);
            let out = nvim.join("lua").join("wolf").join("pin.lua");
            let have = std::fs::read_to_string(&out).unwrap_or_default();
            if have == want {
                eprintln!("nvim: lua/wolf/pin.lua agrees with vendor/upstream/PIN");
            } else if write {
                if let Some(parent) = out.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&out, &want) {
                    Ok(()) => eprintln!("nvim: wrote {}", slash(&out)),
                    Err(e) => errors.push(format!("{}: {e}", slash(&out))),
                }
            } else {
                errors.push(format!(
                    "{} is stale — regenerate with `cargo xtask nvim-generate` \
                     (it is GENERATED from vendor/upstream/PIN; :checkhealth wolf compares a \
                     user's `wolf --version` against it)",
                    slash(&out)
                ));
            }
        }
        Err(e) => errors.push(format!("{}: {e}", slash(&pin_path))),
    }

    report("nvim-check", errors)
}

/// `reserved_kw ::= 'as' | 'asm' | …` → the set of quoted terminals.
///
/// Hand-parsed for the same reason `sample_paths` is: xtask depends on no
/// workspace crate and no regex engine, so that it still runs when the harness
/// does not compile.
pub(crate) fn reserved_keywords(ebnf: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_rule = false;
    for line in ebnf.lines() {
        if line.starts_with("reserved_kw") && line.contains("::=") {
            in_rule = true;
        } else if in_rule {
            // The rule ends at the next rule head or a blank line. A
            // continuation line is indented and starts with `|`.
            let trimmed = line.trim_start();
            if trimmed.is_empty()
                || (line.contains("::=") && !line.starts_with(char::is_whitespace))
            {
                break;
            }
            if !trimmed.starts_with('|') {
                break;
            }
        }
        if !in_rule {
            continue;
        }
        for token in quoted(line) {
            out.insert(token);
        }
    }
    out
}

/// The keyword set `syntax/wolf.vim` declares between its drift markers.
///
/// Returns `None` when the markers are absent, which is a failure and not an
/// empty set: a check that silently passes when its input disappears is worse
/// than no check.
fn syntax_keywords(vim: &str) -> Option<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    let mut inside = false;
    let mut saw_markers = false;
    for line in vim.lines() {
        let trimmed = line.trim();
        if trimmed.ends_with("reserved-kw-begin") {
            inside = true;
            saw_markers = true;
            continue;
        }
        if trimmed.ends_with("reserved-kw-end") {
            inside = false;
            continue;
        }
        if !inside {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("syn keyword ") else {
            continue;
        };
        // `syn keyword <group> tok tok tok`. The first word is the highlight
        // group; the split into groups is cosmetic and the union is the claim.
        for word in rest.split_whitespace().skip(1) {
            // Stop at any `syn-keyword` argument (`contained`, `nextgroup=…`).
            if word.contains('=') || word == "contained" {
                break;
            }
            out.insert(word.to_string());
        }
    }
    saw_markers.then_some(out)
}

/// Every `'…'` or `"…"` terminal on a line, in order.
///
/// Both quote styles are terminals in the vendored EBNF: a terminal that IS
/// an apostrophe is spelled `"'"` and one that is a double quote is spelled
/// `'"'` (`[gram.lex.char]`'s `CHAR_LIT`/`CHAR_ESC`, the 83f83bb pin). A
/// single-quote-only reader lexed the text BETWEEN `"'"` and `"'"` as one
/// token, so `(CHAR_TEXT | CHAR_ESC)` leaked into the operator alternation.
pub(crate) fn quoted(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        let delim = match c {
            '\'' | '"' => c,
            _ => continue,
        };
        let token: String = chars.by_ref().take_while(|c| *c != delim).collect();
        if !token.is_empty() {
            out.push(token);
        }
    }
    out
}

/// One `key = value` out of the PIN file, comments stripped, quotes trimmed.
///
/// Shared rather than re-closured per caller: `compat.rs` and `release.rs` both
/// key their whole output off `commit` and `version`, and three copies of a
/// hand-rolled reader is how two of them end up disagreeing about quoting.
pub(crate) fn pin_value(pin: &str, key: &str) -> String {
    pin.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
        .unwrap_or_default()
}

/// Render `lua/wolf/pin.lua` from the PIN file's text.
fn pin_lua(pin: &str) -> String {
    let value = |key: &str| -> String { pin_value(pin, key) };
    format!(
        r#"-- GENERATED by `cargo xtask nvim-generate` from vendor/upstream/PIN.
-- Do not edit; `cargo xtask nvim-check` fails on a hand edit.
--
-- The wolf-lang build this plugin was verified against. `:checkhealth wolf`
-- compares a user's `wolf --version` against `version` and reports a mismatch
-- WITHOUT calling it unsupported: a supported version RANGE is ls07's to
-- define, and this repo's only honest fact today is one exact pin.
return {{
  commit = "{commit}",
  version = "{version}",
  serves_lsp = {serves_lsp},
}}
"#,
        commit = value("commit"),
        version = value("version"),
        serves_lsp = value("serves_lsp"),
    )
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
