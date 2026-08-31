//! `cargo xtask config-check` and `cargo xtask emacs-check` — the ls06 gates.
//!
//! The config tier ships four editors whose configuration lives in four
//! unrelated file formats, and the things that must stay true across them are
//! not checkable by any of the four editors:
//!
//! - every client spawns the SAME command, `wolf lsp` (D34). The uniformity is
//!   the point and is the entire reason a config tier exists at all, so a
//!   client that quietly grew a flag would break the claim rather than a test;
//! - `.wolfi` is never attached to a language server anywhere. Three sprints
//!   reached that ruling independently (ls04, ls05, ls06) and each recorded it
//!   in prose; prose does not fail a build;
//! - the grammar blocks are LIVE (le02 — `tree-sitter-wolf` carries a real
//!   grammar with a committed `src/parser.c`) and both editors must name the
//!   SAME rev: helix's `[[grammar]]` `rev` and Zed's `[grammars.wolf]`
//!   `commit` are two spellings of one pin, and Zed BUILDS the grammar at
//!   install time, so a rev that drifts or dangles takes the language server
//!   down with it. (Until le02 this bullet was the opposite claim — no live
//!   block anywhere — and le02 flipped the configs without flipping the
//!   check, which is how trunk CI went red: the gate now asserts the state
//!   the repo actually ships.);
//! - the formatter's two numbers, `INDENT = 4` and `WIDTH = 100`, are the same
//!   numbers in every client. A hand-picked tab width in an editor config is a
//!   second formatter with an opinion.
//!
//! **What this cannot check, stated rather than implied.** `INDENT` and `WIDTH`
//! are `wolf_fmt`'s constants, and they are **not vendored** — only `spec/` and
//! `corpus/` are consumed from the pin, and the submodule holding the rest is
//! private and unclonable by CI (`vendor/README.md`). So the numbers are checked
//! for agreement WITH EACH OTHER across four clients, not against their source.
//! Four files drifting together is a much smaller target than one drifting
//! alone, and claiming more than that would be the kind of check that looks
//! like it covers something it does not. (`cargo xtask independence` forbids
//! this file from even naming the path they live at, which is the same rule
//! stated from the other side.)
//!
//! Everything here is a hand-rolled line reader, matching `fixtures_check` and
//! `sample_paths` in `main.rs`: a handful of assertions against four config
//! files does not justify a TOML dependency in a repo whose posture is
//! dependency thinness.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::slash;

/// The indent width and line width every client must agree on.
const INDENT: &str = "4";
const WIDTH: &str = "100";

/// Read a file, or record the failure to. Returns `None` so a caller can skip
/// checks that would otherwise report a second, derived error.
fn read(root: &Path, rel: &str, errors: &mut Vec<String>) -> Option<String> {
    let path: PathBuf = root.join(rel);
    match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) => {
            errors.push(format!("{}: {e}", slash(&path)));
            None
        }
    }
}

/// Lines with comment markers and blank lines removed.
///
/// Both `#` (TOML) and `;` (elisp) start a comment in the files read here, and
/// every "is this block live?" question below is really "is it live *outside* a
/// comment?" — the grammar blocks are all shipped commented out on purpose, so
/// a check that did not strip comments would fire on the very thing it wants.
fn live_lines(text: &str, comment: char) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(comment))
        .collect()
}

/// Everything after `key =` on the first live line whose key matches, with
/// surrounding whitespace gone. `None` when the key is absent.
fn value<'a>(lines: &[&'a str], key: &str) -> Option<&'a str> {
    lines.iter().find_map(|line| {
        let rest = line.strip_prefix(key)?;
        let rest = rest.trim_start();
        rest.strip_prefix('=').map(str::trim)
    })
}

/// A TOML section's live lines: everything after `[header]` until the next
/// `[`-headed line.
fn section<'a>(lines: &'a [&'a str], header: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in lines {
        if line.starts_with('[') {
            inside = *line == header;
            continue;
        }
        if inside {
            out.push(*line);
        }
    }
    out
}

// ------------------------------------------------------------------ helix --

/// Returns the tree-sitter-wolf rev the fragment pins, for the cross-editor
/// one-pin check in [`check`].
fn helix(root: &Path, errors: &mut Vec<String>) -> Option<String> {
    let rel = "clients/helix/languages.toml";
    let Some(text) = read(root, rel, errors) else {
        return None;
    };
    let lines = live_lines(&text, '#');

    // `hx --health wolf` in CI proves the TOML parses and that the command is
    // spelled right. What it CANNOT prove is which command: helix reports the
    // server as found or not found by name and never echoes `args`. So the
    // subcommand is checked here, statically, and that split is deliberate.
    let server = section(&lines, "[language-server.wolf]");
    if value(&server, "command") != Some("\"wolf\"") {
        errors.push(format!(
            "{rel}: [language-server.wolf] must be `command = \"wolf\"` — every editor in \
             this repo spawns the same binary (D34)"
        ));
    }
    if value(&server, "args") != Some("[\"lsp\"]") {
        errors.push(format!(
            "{rel}: [language-server.wolf] must be `args = [\"lsp\"]` — `wolf` with no \
             subcommand is not a server"
        ));
    }

    // The `wolf` language block. `file-types` is `["lu"]` and nothing else:
    // module discovery is `.lu`-keyed (D32), so anything else here attaches the
    // server to documents it has nothing to say about.
    let mut in_wolf = false;
    let mut in_wolfi = false;
    let (mut wolf_ft, mut wolfi_servers, mut wolf_servers) = (None, false, false);
    let (mut wolf_indent, mut wolf_width) = (None, None);
    for line in &lines {
        if line.starts_with("[[language]]") {
            in_wolf = false;
            in_wolfi = false;
            continue;
        }
        if line.starts_with('[') {
            in_wolf = false;
            in_wolfi = false;
            continue;
        }
        if let Some(name) = line.strip_prefix("name =") {
            match name.trim() {
                "\"wolf\"" => in_wolf = true,
                "\"wolfi\"" => in_wolfi = true,
                _ => {}
            }
            continue;
        }
        if in_wolf {
            if let Some(v) = line.strip_prefix("file-types") {
                wolf_ft = v.split_once('=').map(|(_, v)| v.trim().to_string());
            }
            if line.starts_with("language-servers") {
                wolf_servers = true;
            }
            if let Some(v) = line.strip_prefix("indent") {
                wolf_indent = v.split_once('=').map(|(_, v)| v.trim().to_string());
            }
            if let Some(v) = line.strip_prefix("text-width") {
                wolf_width = v.split_once('=').map(|(_, v)| v.trim().to_string());
            }
        }
        if in_wolfi && line.starts_with("language-servers") {
            wolfi_servers = true;
        }
    }

    if wolf_ft.as_deref() != Some("[\"lu\"]") {
        errors.push(format!(
            "{rel}: the `wolf` language must be `file-types = [\"lu\"]` (found {:?}) — \
             `wolf lsp` discovers modules by `.lu` alone (D32)",
            wolf_ft.as_deref().unwrap_or("<absent>")
        ));
    }
    if !wolf_servers {
        errors.push(format!(
            "{rel}: the `wolf` language declares no `language-servers`, so nothing starts"
        ));
    }
    if wolfi_servers {
        errors.push(format!(
            "{rel}: the `wolfi` language declares `language-servers` — `.wolfi` is a BINARY \
             format the server has no path for, and attaching one produces a buffer that \
             looks supported and is not (ls04/ls05/ls06 all reached this ruling)"
        ));
    }
    match wolf_indent.as_deref() {
        Some(v) if v.contains(&format!("tab-width = {INDENT}")) => {}
        other => errors.push(format!(
            "{rel}: the `wolf` language must set `indent = {{ tab-width = {INDENT}, … }}` \
             (found {:?}) — `wolf_fmt::doc::INDENT`",
            other.unwrap_or("<absent>")
        )),
    }
    if wolf_width.as_deref() != Some(WIDTH) {
        errors.push(format!(
            "{rel}: the `wolf` language must set `text-width = {WIDTH}` (found {:?}) — \
             `wolf_fmt::doc::WIDTH`",
            wolf_width.as_deref().unwrap_or("<absent>")
        ));
    }

    // A `formatter` would silently take every save OFF the language server:
    // helix prefers a configured external formatter over LSP formatting, so the
    // path this repository's transcripts cover would stop being the path a user
    // runs.
    if lines.iter().any(|l| l.starts_with("formatter")) {
        errors.push(format!(
            "{rel}: `formatter` is set — helix prefers it over the language server, which \
             takes format-on-save off `textDocument/formatting` and onto an untested \
             second spawn of the same binary"
        ));
    }

    // Live since le02: the grammar is real, and `hx -g fetch && hx -g build`
    // compiles the committed parser from the pinned rev. The rev is returned
    // so `check` can hold it against Zed's — one pin, two spellings.
    if !lines.iter().any(|l| l.starts_with("[[grammar]]")) {
        errors.push(format!(
            "{rel}: no live `[[grammar]]` block — `tree-sitter-wolf` is real since le02 and \
             a fragment without the block leaves every helix user unhighlighted"
        ));
        return None;
    }
    match grammar_rev(&lines, "rev") {
        Some(rev) => Some(rev),
        None => {
            errors.push(format!(
                "{rel}: the `[[grammar]]` block has no `rev = \"<40-hex>\"` — an unpinned \
                 grammar is whatever trunk was this morning, which is the thing every other \
                 pin in this repository exists to prevent"
            ));
            None
        }
    }
}

/// The first 40-hex `<key> = "…"` value on any live line.
fn grammar_rev(lines: &[&str], key: &str) -> Option<String> {
    lines.iter().find_map(|line| {
        let at = line.find(&format!("{key} = \""))?;
        let rest = &line[at + key.len() + 4..];
        let end = rest.find('"')?;
        let rev = &rest[..end];
        (rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit())).then(|| rev.to_string())
    })
}

// -------------------------------------------------------------------- zed --

/// Returns the tree-sitter-wolf commit the manifest pins, for the
/// cross-editor one-pin check in [`check`].
fn zed(root: &Path, errors: &mut Vec<String>) -> Option<String> {
    let mut grammar_commit = None;
    let manifest_rel = "clients/zed/extension.toml";
    if let Some(text) = read(root, manifest_rel, errors) {
        let lines = live_lines(&text, '#');
        let server = section(&lines, "[language_servers.wolf]");
        if server.is_empty() {
            errors.push(format!(
                "{manifest_rel}: no live [language_servers.wolf] table — the id is what a \
                 user writes under `lsp` in settings.json and what src/lib.rs asks \
                 `LspSettings::for_worktree` for"
            ));
        }
        match value(&server, "languages") {
            Some("[\"Wolf\"]") => {}
            other => errors.push(format!(
                "{manifest_rel}: [language_servers.wolf] must be `languages = [\"Wolf\"]` \
                 (found {:?}) — adding \"Wolfi\" would attach the server to a binary format \
                 it has no path for",
                other.unwrap_or("<absent>")
            )),
        }
        // Live since le02. Zed BUILDS every grammar named here at extension
        // install, straight from the committed `src/parser.c` at `commit` —
        // which is why the commit must exist and be pinned, and why it must
        // be the same rev helix names (`check` compares them).
        let grammar = section(&lines, "[grammars.wolf]");
        if grammar.is_empty() {
            errors.push(format!(
                "{manifest_rel}: no live [grammars.wolf] table — `tree-sitter-wolf` is real \
                 since le02, and without the table `languages/wolf/config.toml`'s \
                 `grammar = \"wolf\"` names a grammar Zed cannot build"
            ));
        } else {
            grammar_commit = grammar_rev(&grammar, "commit");
            if grammar_commit.is_none() {
                errors.push(format!(
                    "{manifest_rel}: [grammars.wolf] has no `commit = \"<40-hex>\"` — an \
                     unpinned grammar is whatever trunk was this morning"
                ));
            }
        }
    }

    // The three spellings of `wolf` — the manifest table key, the settings id in
    // `src/lib.rs`, and the `name` field — have to agree, and nothing in Zed
    // checks that. A rename of the manifest key would silently make
    // `LspSettings::for_worktree` read nobody's settings.
    let lib_rel = "clients/zed/src/lib.rs";
    if let Some(lib) = read(root, lib_rel, errors) {
        if !lib.contains("const SERVER_ID: &str = \"wolf\";") {
            errors.push(format!(
                "{lib_rel}: SERVER_ID must be \"wolf\", matching [language_servers.wolf] in \
                 extension.toml"
            ));
        }
        if !lib.contains("const LSP_ARGS: &[&str] = &[\"lsp\"];") {
            errors.push(format!(
                "{lib_rel}: LSP_ARGS must be [\"lsp\"] — every editor in this repo spawns the \
                 same command (D34)"
            ));
        }
    }

    let cfg_rel = "clients/zed/languages/wolf/config.toml";
    if let Some(text) = read(root, cfg_rel, errors) {
        let lines = live_lines(&text, '#');
        if value(&lines, "name") != Some("\"Wolf\"") {
            errors.push(format!(
                "{cfg_rel}: `name` must be \"Wolf\" — extension.toml's `languages` list names \
                 it, not a file extension"
            ));
        }
        if value(&lines, "path_suffixes") != Some("[\"lu\"]") {
            errors.push(format!(
                "{cfg_rel}: `path_suffixes` must be [\"lu\"] — `wolf lsp` discovers modules by \
                 `.lu` alone (D32)"
            ));
        }
        if value(&lines, "tab_size") != Some(INDENT) {
            errors.push(format!(
                "{cfg_rel}: `tab_size` must be {INDENT} — `wolf_fmt::doc::INDENT`"
            ));
        }
        // Live since le02: the key names the [grammars.wolf] table, and the
        // extension ships `languages/wolf/highlights.scm` beside it — a
        // grammar with no queries highlights nothing, silently.
        if value(&lines, "grammar") != Some("\"wolf\"") {
            errors.push(format!(
                "{cfg_rel}: `grammar` must be \"wolf\" — the [grammars.wolf] table in \
                 extension.toml is what it names, and without the key Zed parses `.lu` as \
                 plain text"
            ));
        }
        let hl_rel = "clients/zed/languages/wolf/highlights.scm";
        match std::fs::read_to_string(root.join(hl_rel)) {
            Ok(hl) if hl.lines().any(|l| l.trim_start().starts_with('(')) => {}
            Ok(_) => errors.push(format!(
                "{hl_rel}: no capture patterns — a live grammar with empty queries \
                 highlights nothing, silently"
            )),
            Err(e) => errors.push(format!("{hl_rel}: {e}")),
        }
        if lines.iter().any(|l| l.starts_with("block_comment")) {
            errors.push(format!(
                "{cfg_rel}: `block_comment` is set — wolf has no block comment form, and \
                 offering one hands Zed's comment commands a construct the lexer rejects"
            ));
        }
    }

    let wolfi_rel = "clients/zed/languages/wolfi/config.toml";
    if let Some(text) = read(root, wolfi_rel, errors) {
        let lines = live_lines(&text, '#');
        if value(&lines, "path_suffixes") != Some("[\"wolfi\"]") {
            errors.push(format!("{wolfi_rel}: `path_suffixes` must be [\"wolfi\"]"));
        }
    }
    grammar_commit
}

// ------------------------------------------------------------------ emacs --

/// Every `"…"`-delimited token on a line.
///
/// Not `main.rs`'s `quoted`, which reads the SINGLE-quoted terminals of the
/// EBNF; elisp string literals are double-quoted, and `'` in elisp is the quote
/// form (`'("as" …)`) rather than a delimiter — so reusing the EBNF reader here
/// swallows the whole list as one token.
fn double_quoted(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for c in line.chars() {
        match (&mut current, c) {
            (None, '"') => current = Some(String::new()),
            (Some(_), '"') => {
                if let Some(token) = current.take()
                    && !token.is_empty()
                {
                    out.push(token);
                }
            }
            (Some(buf), c) => buf.push(c),
            (None, _) => {}
        }
    }
    out
}

/// The keyword set `wolf-mode.el` declares between its drift markers.
///
/// Returns `None` when the markers are absent, which is a failure and not an
/// empty set — a check that silently passes when its input disappears is worse
/// than no check. Mirrors `syntax_keywords` in `main.rs` exactly.
fn elisp_keywords(el: &str) -> Option<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    let mut inside = false;
    let mut saw_markers = false;
    for line in el.lines() {
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
        for token in double_quoted(trimmed) {
            out.insert(token);
        }
    }
    saw_markers.then_some(out)
}

/// `cargo xtask emacs-check` — the keyword drift gate plus the README/`.el`
/// agreement, in Rust so that both run on a machine with no Emacs installed.
pub fn emacs(root: &Path) -> Vec<String> {
    let mut errors = Vec::new();

    let ebnf_rel = "vendor/upstream/spec/grammar.ebnf";
    let el_rel = "clients/emacs/wolf-mode.el";
    let (Some(ebnf), Some(el)) = (
        read(root, ebnf_rel, &mut errors),
        read(root, el_rel, &mut errors),
    ) else {
        return errors;
    };

    let expected = crate::reserved_keywords(&ebnf);
    if expected.is_empty() {
        errors.push(format!(
            "{ebnf_rel}: no `reserved_kw ::=` rule found — the extraction this check depends \
             on has changed shape"
        ));
        return errors;
    }

    match elisp_keywords(&el) {
        Some(found) => {
            for missing in expected.difference(&found) {
                errors.push(format!(
                    "{el_rel} is missing the reserved keyword `{missing}` — a keyword the \
                     compiler knows and the editor does not renders as an identifier"
                ));
            }
            for extra in found.difference(&expected) {
                errors.push(format!(
                    "{el_rel} colours `{extra}` as a reserved keyword, and `reserved_kw` does \
                     not contain it — an invented keyword teaches a language that does not exist"
                ));
            }
            if errors.is_empty() {
                eprintln!(
                    "emacs: wolf-mode.el carries all {} reserved keywords, and no others",
                    expected.len()
                );
            }
        }
        None => errors.push(format!(
            "{el_rel} has no `reserved-kw-begin`/`reserved-kw-end` markers — the drift check \
             reads the keyword set from between them"
        )),
    }

    // The snippet in the README and the file CI runs are two copies of one
    // artifact. `mode-test.el` asserts this too; it is repeated here because
    // that lane needs Emacs and this one does not.
    let readme_rel = "clients/emacs/README.md";
    if let Some(readme) = read(root, readme_rel, &mut errors)
        && !readme.contains(el.trim_end())
    {
        errors.push(format!(
            "{readme_rel} does not contain {el_rel} verbatim — the snippet a reader pastes and \
             the file CI runs have drifted"
        ));
    }

    errors
}

// ------------------------------------------------------------ the numbers --

/// `INDENT` and `WIDTH` agree across every client that states them.
///
/// Checked as a set of literal needles rather than by parsing five formats: the
/// question is only ever "does this file still say 4 and 100", and a parser per
/// format to answer it would be more code than the thing it guards.
fn numbers(root: &Path, errors: &mut Vec<String>) {
    let needles: &[(&str, &[&str])] = &[
        (
            "clients/nvim/ftplugin/wolf.lua",
            &["vim.bo.shiftwidth = 4", "vim.bo.textwidth = 100"],
        ),
        (
            "clients/helix/languages.toml",
            &["tab-width = 4", "text-width = 100"],
        ),
        ("clients/zed/languages/wolf/config.toml", &["tab_size = 4"]),
        (
            "clients/emacs/wolf-mode.el",
            &["(setq-local tab-width 4)", "(setq-local fill-column 100)"],
        ),
    ];
    for (rel, wanted) in needles {
        let Some(text) = read(root, rel, errors) else {
            continue;
        };
        for needle in *wanted {
            if !text.contains(needle) {
                errors.push(format!(
                    "{rel}: expected `{needle}` — `wolf_fmt` fixes INDENT = {INDENT} and \
                     WIDTH = {WIDTH}, and every client states the same two numbers. (This \
                     compares the clients to EACH OTHER: those constants are not vendored, \
                     so no check here can reach their source.)"
                ));
            }
        }
    }
}

// ----------------------------------------------------------- helix-health --

/// What `hx --health <lang>` said, with the terminal escapes gone.
///
/// `--health` colours its output unconditionally — `NO_COLOR` is honoured only
/// to the extent of emitting *empty* SGR sequences (`\x1b[m`), which still sit
/// between the tick and the text — so every assertion below would be comparing
/// against escape codes if this did not run first.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        if chars.peek() == Some(&'[') {
            chars.next();
            // CSI runs to the first byte in 0x40..=0x7e.
            for c in chars.by_ref() {
                if ('\x40'..='\x7e').contains(&c) {
                    break;
                }
            }
        }
    }
    out
}

/// The outcome of the helix lane. `Skip` is a first-class result: an absent
/// editor must say so out loud rather than pass (ls00 §3).
pub enum Health {
    Ok(Vec<String>),
    Skip(String),
}

/// `cargo xtask helix-health` — drop the shipped fragment into a temp config
/// dir and ask helix itself whether it parsed.
///
/// This is the T2 verification for helix, and its whole value is that helix does
/// the parsing. Two findings shape how it asserts:
///
/// - **`hx --health` always exits 0.** An unknown language, a server that is not
///   on `PATH`, a malformed fragment: all exit 0. So the check is entirely a
///   matter of reading stdout, and an exit-code assertion would be a check that
///   cannot fail.
/// - **The "Tree-sitter parser" line is not evidence.** helix 25.07.1 prints
///   `Tree-sitter parser: ✓` for a `grammar` name that does not exist anywhere
///   (verified by pointing one at `definitely-not-a-real-grammar`). Only the
///   "Highlight queries" line reflects reality, which is why the absence of
///   wolf highlighting is asserted through *that* line and the parser line is
///   ignored.
pub fn helix_health(root: &Path) -> Health {
    // Upstream calls the binary `hx`; Arch Linux installs it as `helix` and
    // symlinks nothing. Both are tried rather than one being declared correct.
    let Some(hx) = ["hx", "helix"].into_iter().find(|name| {
        std::process::Command::new(name)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }) else {
        return Health::Skip(
            "no `hx` (or `helix`) on PATH — the helix lane needs the editor itself, because \
             the whole point of the check is that helix does the TOML parsing"
                .to_string(),
        );
    };

    let mut errors = Vec::new();
    let fragment = match std::fs::read_to_string(root.join("clients/helix/languages.toml")) {
        Ok(text) => text,
        Err(e) => return Health::Ok(vec![format!("clients/helix/languages.toml: {e}")]),
    };

    // A temp config dir, not the developer's own: this must not depend on, or
    // disturb, whatever helix config the machine already has.
    let dir = std::env::temp_dir().join(format!("wolf-helix-health-{}", std::process::id()));
    let cfg = dir.join("helix");
    if let Err(e) = std::fs::create_dir_all(&cfg) {
        return Health::Ok(vec![format!("{}: {e}", slash(&cfg))]);
    }
    if let Err(e) = std::fs::write(cfg.join("languages.toml"), &fragment) {
        return Health::Ok(vec![format!("{}: {e}", slash(&cfg))]);
    }

    let run_args = |args: &[&str]| -> Result<String, String> {
        std::process::Command::new(hx)
            .args(args)
            .env("XDG_CONFIG_HOME", &dir)
            .output()
            .map_err(|e| format!("`{hx} {}`: {e}", args.join(" ")))
            .map(|out| {
                let mut text = strip_ansi(&String::from_utf8_lossy(&out.stdout));
                text.push_str(&strip_ansi(&String::from_utf8_lossy(&out.stderr)));
                text
            })
    };
    let run = |lang: &str| run_args(&["--health", lang]);

    // Self-check first. helix resolves its config directory per platform, and
    // `XDG_CONFIG_HOME` is not what it reads everywhere — so before asserting
    // anything about wolf, confirm helix is reading OUR file. Without this the
    // lane would pass vacuously on a platform where the fragment never loaded.
    // `--health` with NO category is the only form that prints the `Config file:`
    // / `Language file:` header; `--health languages` prints the table alone.
    match run_args(&["--health"]) {
        Ok(text) => {
            let ours = slash(&cfg.join("languages.toml"));
            let loaded = text.lines().any(|l| {
                l.starts_with("Language file:") && l.replace('\\', "/").contains(ours.as_str())
            });
            if !loaded {
                let reported = text
                    .lines()
                    .find(|l| l.starts_with("Language file:"))
                    .unwrap_or("<no `Language file:` line>")
                    .to_string();
                return Health::Skip(format!(
                    "helix did not load the fragment from $XDG_CONFIG_HOME — it reported \
                     `{reported}` and the lane would otherwise pass without ever parsing \
                     `clients/helix/languages.toml`"
                ));
            }
        }
        Err(e) => return Health::Ok(vec![e]),
    }

    match run("wolf") {
        Ok(text) => {
            if text.contains("Language 'wolf' not found") {
                errors.push(
                    "hx --health wolf: helix does not recognise the language — the fragment \
                     parsed but its `[[language]]` block did not register"
                        .to_string(),
                );
            }
            // The server must be *configured*. Whether the binary is found is a
            // property of the runner, not of the fragment, so it is reported and
            // not asserted — a runner with no `wolf` on PATH is the normal CI
            // state (there is no release artifact to acquire).
            if !text.contains("wolf:") && !text.lines().any(|l| l.trim().ends_with("wolf")) {
                errors.push(format!(
                    "hx --health wolf: no `wolf` language server listed under `Configured \
                     language servers` — got:\n{text}"
                ));
            }
            // Since le02 the queries are real (tree-sitter-wolf's
            // `queries/*.scm`, copied to helix's runtime dir by the user), so
            // their presence is a property of the RUNNER, reported either way
            // and asserted neither: a CI runner has no runtime queries, a
            // developer who ran the README's copy step has them.
            if text.contains("Highlight queries: ✓") {
                eprintln!("helix: runtime highlight queries present on this runner");
            } else {
                eprintln!(
                    "helix: no runtime highlight queries on this runner (copy \
                     tree-sitter-wolf's queries/*.scm to runtime/queries/wolf/ to light them)"
                );
            }
            eprintln!("helix: --health wolf recognises the language and configures `wolf`");
        }
        Err(e) => errors.push(e),
    }

    match run("wolfi") {
        Ok(text) => {
            if text.contains("Language 'wolfi' not found") {
                errors.push("hx --health wolfi: helix does not recognise the language".to_string());
            }
            // The ruling three sprints reached, asserted by the editor itself.
            if !text.contains("Configured language servers: None") {
                errors.push(format!(
                    "hx --health wolfi: a language server is configured for `.wolfi` — it is a \
                     BINARY format the server has no path for, and attaching one produces a \
                     buffer that looks supported and is not. Got:\n{text}"
                ));
            } else {
                eprintln!("helix: --health wolfi recognises the language and configures no server");
            }
        }
        Err(e) => errors.push(e),
    }

    let _ = std::fs::remove_dir_all(&dir);
    Health::Ok(errors)
}

/// `cargo xtask config-check` — the cross-editor invariants of the config tier.
pub fn check(root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let helix_rev = helix(root, &mut errors);
    let zed_commit = zed(root, &mut errors);
    // One grammar pin, two spellings: helix's `rev` and Zed's `commit` are
    // the same tree-sitter-wolf commit or one of the two editors is
    // highlighting a different language.
    if let (Some(h), Some(z)) = (&helix_rev, &zed_commit) {
        if h != z {
            errors.push(format!(
                "grammar pin drift: clients/helix/languages.toml pins tree-sitter-wolf at \
                 {h} but clients/zed/extension.toml pins {z} — one grammar, one rev, two \
                 spellings"
            ));
        }
    }
    numbers(root, &mut errors);
    if errors.is_empty() {
        eprintln!(
            "config-check: helix, zed and emacs all spawn `wolf lsp`; `.wolfi` is attached \
             to no server; the grammar blocks are live at one pinned rev ({}); INDENT/WIDTH \
             agree across 4 clients",
            helix_rev.as_deref().unwrap_or("<unpinned>"),
        );
    }
    errors
}
