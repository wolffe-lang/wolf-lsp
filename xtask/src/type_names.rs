//! The builtin TYPE list against the compiler — through the acquired binary
//! and the vendored spec, never a source build (wolf-lsp#24).
//!
//! # The hole this closes
//!
//! `builtin_types.rs` (wolf-lsp#21) makes the five copies of the builtin type
//! list agree with EACH OTHER, with [`crate::vscode::TYPE_NAMES`] as the
//! source. Nothing made that source agree with wolfc: `vendor/upstream/`
//! carries no `prelude.rs`, the set is not derivable from the EBNF, and the
//! only thing keeping `TYPE_NAMES` true was a human re-reading
//! `wolf_sema/src/prelude.rs` at each pin bump. A name added upstream left
//! all five artifacts wrong TOGETHER, and every gate stayed green because
//! they agreed — about the wrong set. s158's `range` made that a live case
//! rather than a theoretical one.
//!
//! # What the repo can read without building the compiler
//!
//! Two things, and this gate reads both:
//!
//! 1. **The acquired binary.** `wolf build` on a one-parameter probe is an
//!    oracle for "does this word resolve in type position at the pin":
//!    `error[E0301]: nothing named … is in scope` says NO, a parse-tier
//!    refusal (`E01xx`/`E02xx`) says the word is SYNTAX, and anything else —
//!    a clean build, or a refusal about the name's SHAPE (`range` with no
//!    argument is "a generic application left opaque", not E0301) — says YES.
//!    Measured against `wolf 0.2.14 (wolfgang, pin 30731a6)`: `int`, `byte`,
//!    `char`, `str` and `Self`-inside-an-`impl` build; `range`, `List`,
//!    `Map`, `Pool`, `Mutex`, `channel` and `wrapping` are in scope and refuse
//!    only for lacking an argument; `unit`, `float`, `list`, `map`, `numlit`
//!    are E0301; `fn` and `trait` are E0201/E0202, keywords.
//! 2. **The vendored spec's own index.** `spec/anchors.json` names every
//!    clause, and every type the spec has introduced since anchors began has
//!    a `type.<name>` anchor (`type.byte`, `type.char`, `type.range.name`).
//!    Their second segments are the CANDIDATE set — the words the gate asks
//!    the binary about, over and above the two lists it already holds.
//!
//! So the gate is: every name in `TYPE_NAMES` resolves (a removal or rename
//! upstream turns red); every row in [`crate::vscode::TYPE_POSITION_UNPAINTED`]
//! resolves (a stale exclusion turns red); and every anchor-derived word that
//! resolves and is not a reserved keyword is in one list or the other (a
//! prelude type name added upstream turns red until a human classifies it —
//! the same shape as `CONTEXTUAL` for word terminals). `range` is the first
//! row that rule catches: remove it from `TYPE_POSITION_UNPAINTED` and this
//! gate names it.
//!
//! # The candidate set is wolf's published prelude list (ruling #39 = B)
//!
//! The anchor index above was an approximation, and wolf-lang 0.2.16 broke
//! it: `Scope` and `Proc` arrived anchored `conc.*`, so the gate stayed green
//! across a pin that added two type names (wolf-lsp#32). Ruling #39 = B
//! answered upstream: wolf-lang publishes its prelude as data (s212,
//! `6b5db762`, schema `wolf-prelude/0`, `docs/prelude-json.md`), built from
//! the checker's own tables, and this gate reads it. **The candidate set is
//! every `names[]` entry whose `kind` is `builtin_type` or `type`.** Read
//! from, in order:
//!
//! 1. `vendor/upstream/spec/prelude.json`, vendored beside `anchors.json` at
//!    the pin; when the pinned compiler ALSO answers `wolf prelude --json`,
//!    the two must be the same bytes;
//! 2. else the pinned compiler's `wolf prelude --json`;
//! 3. else — only when the compiler answers that `prelude` is not a command,
//!    i.e. it predates s212 — the anchor index, and the gate SAYS so on its
//!    summary line. Every release up to and including 0.2.23 is such a pin.
//!
//! With the list, the gate proves set EQUALITY rather than membership: every
//! listed type name resolves and is classified, and every name in the two
//! hand lists is listed (`Self` excepted: a context alias, not a prelude
//! name). A list missing `Scope` is as red as a compiler missing it.
//!
//! # What the gate cannot see
//!
//! - At a pin with no list (case 3): a builtin added upstream WITHOUT a
//!   two-segment `type.<name>` anchor of its own spelling — `Scope` and
//!   `Proc` at 0.2.16 were exactly this. The removal half still holds for
//!   such a name once a human has listed it.
//! - With the list: a type-position name the checker resolves through some
//!   path other than its own tables. The list is built from
//!   `PRELUDE`/`BUILTIN_TYPES`, which is what resolution reads, so this is the
//!   compiler disagreeing with itself — and the oracle below would still
//!   resolve the name, but nothing would ask.
//! - Anything at all when no binary at the pin resolves: the oracle half is
//!   server-dependent and exits 77 like `doctor`, so on a box with no
//!   acquired archive the gate is a skip that says so, not a pass.
//! - What the SERVER prints for a name. This gate asks whether a word
//!   resolves; the three type-printing surfaces (hover, inlay hints,
//!   completion detail) are the transcripts' business.
//! - `wolf_sema`'s literal `BUILTIN_TYPES` itself. The gate reads the
//!   compiler's BEHAVIOUR, not its source; a name the compiler resolves in
//!   type position through some path other than that list is a name to the
//!   editor all the same, which is the honest question.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::vscode::{TYPE_NAMES, TYPE_POSITION_UNPAINTED};

/// What the pinned compiler says about a word in type position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Resolves: the probe builds, or refuses for a reason other than the
    /// name (a generic left bare is the common one).
    Name,
    /// `error[E0301]: nothing named … is in scope`.
    NotAName,
    /// A parse-tier refusal — the word is a keyword or punctuation to the
    /// grammar, so the question "is it a name" never reached resolve.
    Syntax,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Verdict::Name => "resolves in type position",
            Verdict::NotAName => "E0301, nothing by that name is in scope",
            Verdict::Syntax => "a parse-tier refusal, the word is syntax",
        })
    }
}

/// The oracle's outcome, or why it could not run.
pub enum Health {
    Ok(Vec<String>),
    Skip(String),
}

/// Read a verdict off `wolf build`'s stderr.
pub(crate) fn verdict_from_stderr(stderr: &str) -> Verdict {
    if stderr.contains("error[E0301]") {
        return Verdict::NotAName;
    }
    let parse_tier = stderr
        .split("error[E")
        .skip(1)
        .filter_map(|rest| rest.get(..4))
        .any(|code| code.starts_with("01") || code.starts_with("02"));
    if parse_tier {
        Verdict::Syntax
    } else {
        Verdict::Name
    }
}

/// The probe: one parameter of the candidate type, inside an `impl` so that
/// `Self` — a context-bound alias, not a prelude name — resolves too. Every
/// other name resolves the same inside and outside the block.
pub(crate) fn probe_source(name: &str) -> String {
    format!(
        "type Probe = struct {{\n    n: int,\n}}\n\nimpl Probe {{\n    fn probe(self, x: {name}) -> int {{\n        0\n    }}\n}}\n\nfn main() -> !int {{\n    0\n}}\n"
    )
}

/// The second segment of every `type.<x>…` anchor in `spec/anchors.json`.
pub(crate) fn type_anchor_segments(anchors_json: &str) -> Result<BTreeSet<String>, String> {
    let v: serde_json::Value = serde_json::from_str(anchors_json).map_err(|e| e.to_string())?;
    let anchors = v
        .get("anchors")
        .and_then(|a| a.as_object())
        .ok_or_else(|| "no top-level `anchors` object".to_string())?;
    Ok(anchors
        .keys()
        .filter_map(|k| {
            let mut parts = k.split('.');
            (parts.next() == Some("type"))
                .then(|| parts.next())
                .flatten()
        })
        .map(str::to_string)
        .collect())
}

/// The schema this gate reads; a removed field or a changed meaning bumps the
/// number upstream (`docs/prelude-json.md` §Versioning), and a gate that read
/// an unknown version would be guessing.
pub(crate) const PRELUDE_SCHEMA: &str = "wolf-prelude/0";

/// The `kind` values `wolf-prelude/0` defines. A new one is not a new FIELD,
/// so the schema number would not move; it is refused here until a human
/// says whether it names a type.
const PRELUDE_KINDS: &[&str] = &[
    "builtin_type",
    "type",
    "function",
    "intrinsic",
    "provisional",
];

/// The kinds that are type-position names — the candidate set.
const PRELUDE_TYPE_KINDS: &[&str] = &["builtin_type", "type"];

/// Names the hand lists hold that no prelude list will: bound by context, not
/// by the prelude.
const CONTEXT_BOUND: &[&str] = &["Self"];

/// The candidate set from a `wolf-prelude/0` document: every `names[]` entry
/// whose `kind` is `builtin_type` or `type`. Unknown keys are ignored, as the
/// schema promises; an unknown `kind` or schema is an error.
pub(crate) fn prelude_type_names(json: &str) -> Result<BTreeSet<String>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    match v.get("schema").and_then(serde_json::Value::as_str) {
        Some(PRELUDE_SCHEMA) => {}
        other => {
            return Err(format!(
                "schema is {other:?}, and this gate reads {PRELUDE_SCHEMA:?} only"
            ));
        }
    }
    let names = v
        .get("names")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "no top-level `names` array".to_string())?;
    let mut out = BTreeSet::new();
    for (i, entry) in names.iter().enumerate() {
        let name = entry.get("name").and_then(serde_json::Value::as_str);
        let kind = entry.get("kind").and_then(serde_json::Value::as_str);
        let (Some(name), Some(kind)) = (name, kind) else {
            return Err(format!("names[{i}] has no string `name` and `kind`"));
        };
        if !PRELUDE_KINDS.contains(&kind) {
            return Err(format!(
                "names[{i}] `{name}` has kind `{kind}`, which {PRELUDE_SCHEMA} did not define — \
                 decide whether it names a type and teach PRELUDE_TYPE_KINDS"
            ));
        }
        if PRELUDE_TYPE_KINDS.contains(&kind) && !out.insert(name.to_string()) {
            return Err(format!("names[{i}] lists `{name}` twice"));
        }
    }
    if out.is_empty() {
        return Err("no `builtin_type` or `type` entry at all".to_string());
    }
    Ok(out)
}

/// Where the candidate set came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Candidates {
    /// wolf's published prelude list; the string says which copy.
    Published {
        names: BTreeSet<String>,
        from: String,
    },
    /// The spec's `type.*` index, because the pin publishes no list; the
    /// string says why.
    Anchors {
        segments: BTreeSet<String>,
        why: String,
    },
}

impl Candidates {
    fn words(&self) -> &BTreeSet<String> {
        match self {
            Candidates::Published { names, .. } => names,
            Candidates::Anchors { segments, .. } => segments,
        }
    }
}

/// What `wolf prelude --json` said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompilerList {
    Answered(String),
    /// The compiler predates s212: `prelude` is not one of its commands.
    NoCommand,
    Failed(String),
}

/// Read the compiler's refusal: only "`prelude` is not a wolf command" means
/// the command does not exist. Any other failure of a command that does is a
/// failure, never a reason to fall back.
pub(crate) fn compiler_list_from(ok: bool, stdout: &str, stderr: &str) -> CompilerList {
    if ok {
        CompilerList::Answered(stdout.to_string())
    } else if stderr.contains("`prelude` is not a wolf command") {
        CompilerList::NoCommand
    } else {
        CompilerList::Failed(stderr.lines().next().unwrap_or("no stderr").to_string())
    }
}

/// Decide the candidate set from the vendored file (if any), the compiler's
/// answer, and the anchor index.
pub(crate) fn choose_candidates(
    vendored: Option<&str>,
    compiler: &CompilerList,
    anchors: impl FnOnce() -> Result<BTreeSet<String>, String>,
) -> Result<Candidates, String> {
    const VENDORED: &str = "vendor/upstream/spec/prelude.json";
    match (vendored, compiler) {
        (Some(text), CompilerList::Answered(out)) if text != out => Err(format!(
            "{VENDORED} and the pinned compiler's `wolf prelude --json` differ — the list is \
             one set of bytes in three copies (docs/prelude-json.md); re-vendor at the pin"
        )),
        (Some(_), CompilerList::NoCommand) => Err(format!(
            "{VENDORED} is vendored but the pinned compiler has no `prelude` command — the file \
             is from a newer wolf-lang than the pin"
        )),
        (_, CompilerList::Failed(e)) => Err(format!("`wolf prelude --json` failed: {e}")),
        (Some(text), _) => Ok(Candidates::Published {
            names: prelude_type_names(text).map_err(|e| format!("{VENDORED}: {e}"))?,
            from: format!("{VENDORED}, {PRELUDE_SCHEMA}"),
        }),
        (None, CompilerList::Answered(out)) => Ok(Candidates::Published {
            names: prelude_type_names(out).map_err(|e| format!("`wolf prelude --json`: {e}"))?,
            from: format!(
                "the pinned compiler's `wolf prelude --json`, {PRELUDE_SCHEMA} ({VENDORED} \
                 arrives at the next re-vendor)"
            ),
        }),
        (None, CompilerList::NoCommand) => Ok(Candidates::Anchors {
            segments: anchors()?,
            why: format!(
                "NO PRELUDE LIST AT THIS PIN: no {VENDORED}, and the pinned compiler has no \
                 `prelude` command (it predates wolf-lang 6b5db762, s212) — a prelude type name \
                 with no `type.<name>` anchor is invisible here, as Scope and Proc were \
                 (wolf-lsp#32)"
            ),
        }),
    }
}

/// The classification, with the oracle abstracted so a test can fake it.
///
/// `reserved` is `reserved_kw` from the vendored EBNF: a keyword can appear
/// as an anchor segment (`type.fn`, `type.trait`) and can never be a type
/// NAME, so it is dropped before the compiler is asked.
pub(crate) fn classify(
    type_names: &[&str],
    unpainted: &[(&str, &str)],
    reserved: &BTreeSet<String>,
    candidates: &Candidates,
    verdict: &mut dyn FnMut(&str) -> Verdict,
) -> Vec<String> {
    let anchor_segments = candidates.words();
    let published = matches!(candidates, Candidates::Published { .. });
    let mut errors = Vec::new();
    let painted: BTreeSet<&str> = type_names.iter().copied().collect();
    let excluded: BTreeSet<&str> = unpainted.iter().map(|(n, _)| *n).collect();

    for (name, reason) in unpainted {
        if reason.trim().is_empty() {
            errors.push(format!(
                "TYPE_POSITION_UNPAINTED lists `{name}` with no reason — the row IS the ruling, \
                 and a ruling with no argument is a guess"
            ));
        }
        if painted.contains(name) {
            errors.push(format!(
                "`{name}` is in both TYPE_NAMES and TYPE_POSITION_UNPAINTED — pick one"
            ));
        }
    }

    let mut candidates: BTreeSet<&str> = painted.iter().copied().collect();
    candidates.extend(excluded.iter().copied());
    // A keyword can be an anchor segment (`type.fn`) and is dropped before the
    // compiler is asked. A published TYPE name is never filtered: if the list
    // ever names a keyword, the oracle answers Syntax and that is a red.
    candidates.extend(
        anchor_segments
            .iter()
            .map(String::as_str)
            .filter(|s| published || !reserved.contains(*s)),
    );
    let verdicts: BTreeMap<&str, Verdict> = candidates.iter().map(|c| (*c, verdict(c))).collect();

    for name in type_names {
        if verdicts[name] != Verdict::Name {
            errors.push(format!(
                "TYPE_NAMES paints `{name}`, and the pinned compiler does not resolve it in type \
                 position ({}) — a builtin type removed or renamed upstream; drop it from \
                 xtask/src/vscode.rs and regenerate the four artifacts",
                verdicts[name]
            ));
        }
    }
    for (name, _) in unpainted {
        if verdicts[name] != Verdict::Name {
            errors.push(format!(
                "TYPE_POSITION_UNPAINTED lists `{name}`, and the pinned compiler does not resolve \
                 it in type position ({}) — a stale exclusion row",
                verdicts[name]
            ));
        }
    }
    if published {
        // EQUALITY, not membership: the hand lists may hold nothing the
        // published list does not, so a name dropped upstream (or a list that
        // lost one) is red even while the compiler still resolves it.
        for name in type_names
            .iter()
            .chain(unpainted.iter().map(|(n, _)| n))
            .filter(|n| !CONTEXT_BOUND.contains(*n))
        {
            if !anchor_segments.contains(*name) {
                errors.push(format!(
                    "xtask lists `{name}`, and wolf's published prelude list does not name it as \
                     a type (kind builtin_type or type) — the list and the hand lists must be \
                     the same set"
                ));
            }
        }
    }
    for seg in anchor_segments {
        let seg = seg.as_str();
        if published && verdicts[seg] != Verdict::Name {
            errors.push(format!(
                "wolf's published prelude list names `{seg}` as a type, and the pinned compiler \
                 does not resolve it in type position ({}) — the list and the compiler disagree",
                verdicts[seg]
            ));
            continue;
        }
        if reserved.contains(seg) || painted.contains(seg) || excluded.contains(seg) {
            continue;
        }
        if published {
            errors.push(format!(
                "wolf's published prelude list names `{seg}` as a type and the pinned compiler \
                 resolves it, but xtask has not classified it: add it to TYPE_NAMES (a bare \
                 builtin scalar the four regular artifacts paint) or to TYPE_POSITION_UNPAINTED \
                 with the reason"
            ));
        } else if verdicts[seg] == Verdict::Name {
            errors.push(format!(
                "the pinned spec anchors `type.{seg}` and the pinned compiler resolves `{seg}` in \
                 type position, but xtask has not classified it: add it to TYPE_NAMES (a bare \
                 builtin scalar the four regular artifacts paint) or to TYPE_POSITION_UNPAINTED \
                 with the reason (a prelude name that takes an argument, or is bound in type \
                 position only — the shape `range` has)"
            ));
        }
    }
    errors
}

// ---------------------------------------------------------------- the binary --

fn exe_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

/// ls00 §3's order, mirrored from `lsp_harness::locate` (xtask must build when
/// the harness does not): `$WOLF_BIN` → `.wolf-bin/` → `PATH`.
fn locate(root: &Path) -> Option<PathBuf> {
    if let Some(raw) = std::env::var_os("WOLF_BIN") {
        let p = PathBuf::from(raw);
        if p.is_file() {
            return Some(p);
        }
    }
    let cached = root.join(".wolf-bin").join(exe_name("wolf"));
    if cached.is_file() {
        return Some(cached);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(exe_name("wolf")))
        .find(|p| p.is_file())
}

fn pin_version(root: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(root.join("vendor").join("upstream").join("PIN"))
        .map_err(|e| format!("vendor/upstream/PIN: {e}"))?;
    text.lines()
        .filter_map(|raw| raw.split('#').next())
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| k.trim() == "version")
        .map(|(_, v)| v.trim().trim_matches('"').to_string())
        .ok_or_else(|| "vendor/upstream/PIN has no `version` line".to_string())
}

fn ask(wolf: &Path, scratch: &Path, name: &str) -> Result<Verdict, String> {
    let dir = scratch.join(name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let src = dir.join("main.lu");
    std::fs::write(&src, probe_source(name)).map_err(|e| format!("{}: {e}", src.display()))?;
    let out = Command::new(wolf)
        .arg("build")
        .arg("--emit=wir")
        .arg(&src)
        .arg("-o")
        .arg(dir.join("out.wir"))
        .output()
        .map_err(|e| format!("{}: {e}", wolf.display()))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    Ok(verdict_from_stderr(&stderr))
}

/// The gate. A missing binary is a skip; a binary at the wrong version, a
/// broken oracle, or a misclassified name is a failure.
pub fn check(root: &Path) -> Health {
    let Some(wolf) = locate(root) else {
        return Health::Skip(
            "no `wolf` resolves ($WOLF_BIN, .wolf-bin/, PATH) — the compiler half of the gate \
             needs the acquired archive at the pin"
                .to_string(),
        );
    };
    let expected = match pin_version(root) {
        Ok(v) => v,
        Err(e) => return Health::Ok(vec![e]),
    };
    let found = match Command::new(&wolf).arg("--version").output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string(),
        Err(e) => return Health::Ok(vec![format!("{}: {e}", wolf.display())]),
    };
    if found != expected {
        return Health::Ok(vec![format!(
            "{} reports {found:?} but the pin expects {expected:?} — testing the wrong binary \
             is worse than testing none",
            wolf.display()
        )]);
    }

    let vendor = root.join("vendor").join("upstream").join("spec");
    let ebnf = match std::fs::read_to_string(vendor.join("grammar.ebnf")) {
        Ok(t) => t,
        Err(e) => return Health::Ok(vec![format!("vendor/upstream/spec/grammar.ebnf: {e}")]),
    };
    let reserved = crate::reserved_keywords(&ebnf);
    if reserved.is_empty() {
        return Health::Ok(vec![
            "no `reserved_kw ::=` rule in the vendored grammar — the keyword filter cannot run"
                .to_string(),
        ]);
    }
    let vendored = match std::fs::read_to_string(vendor.join("prelude.json")) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Health::Ok(vec![format!("vendor/upstream/spec/prelude.json: {e}")]),
    };
    let compiler = match Command::new(&wolf).args(["prelude", "--json"]).output() {
        Ok(o) => compiler_list_from(
            o.status.success(),
            &String::from_utf8_lossy(&o.stdout),
            &String::from_utf8_lossy(&o.stderr),
        ),
        Err(e) => CompilerList::Failed(format!("{}: {e}", wolf.display())),
    };
    let candidates = match choose_candidates(vendored.as_deref(), &compiler, || {
        std::fs::read_to_string(vendor.join("anchors.json"))
            .map_err(|e| e.to_string())
            .and_then(|t| type_anchor_segments(&t))
            .map_err(|e| format!("vendor/upstream/spec/anchors.json: {e}"))
    }) {
        Ok(c) => c,
        Err(e) => return Health::Ok(vec![e]),
    };

    let scratch = std::env::temp_dir().join(format!("wolf-lsp-type-names-{}", std::process::id()));
    let mut errors = Vec::new();
    // The oracle proves it can answer both ways before anything trusts it: a
    // binary that cannot find its runtime would refuse every probe alike, and
    // "no E0301 anywhere" would read as "every word is a name".
    match (
        ask(&wolf, &scratch, "int"),
        ask(&wolf, &scratch, "__wolf_lsp_no_such_type"),
    ) {
        (Ok(Verdict::Name), Ok(Verdict::NotAName)) => {}
        (a, b) => errors.push(format!(
            "the type-position oracle is broken and nothing below can be trusted: `int` answered \
             {a:?}, an unspellable name answered {b:?}"
        )),
    }
    if errors.is_empty() {
        let mut verdict = |name: &str| match ask(&wolf, &scratch, name) {
            Ok(v) => v,
            Err(e) => {
                errors.push(e);
                Verdict::NotAName
            }
        };
        let found = classify(
            TYPE_NAMES,
            TYPE_POSITION_UNPAINTED,
            &reserved,
            &candidates,
            &mut verdict,
        );
        errors.extend(found);
    }
    let _ = std::fs::remove_dir_all(&scratch);
    match &candidates {
        Candidates::Published { from, .. } => eprintln!("type-names: candidates from {from}"),
        Candidates::Anchors { why, .. } => {
            eprintln!("type-names: {why}; candidates from the spec's type.* index instead");
        }
    }
    if errors.is_empty() {
        let (n, what) = match &candidates {
            Candidates::Published { names, .. } => (names.len(), "prelude-list candidates"),
            Candidates::Anchors { segments, .. } => (segments.len(), "anchor candidates"),
        };
        eprintln!(
            "type-names: {} painted, {} unpainted, {n} {what}, all classified against {}",
            TYPE_NAMES.len(),
            TYPE_POSITION_UNPAINTED.len(),
            found
        );
    }
    Health::Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake<'a>(names: &'a [&'a str], syntax: &'a [&'a str]) -> impl FnMut(&str) -> Verdict + 'a {
        move |w: &str| {
            if syntax.contains(&w) {
                Verdict::Syntax
            } else if names.contains(&w) {
                Verdict::Name
            } else {
                Verdict::NotAName
            }
        }
    }

    fn set(words: &[&str]) -> BTreeSet<String> {
        words.iter().map(|s| (*s).to_string()).collect()
    }

    fn anchors(words: &[&str]) -> Candidates {
        Candidates::Anchors {
            segments: set(words),
            why: "a test".to_string(),
        }
    }

    fn published(words: &[&str]) -> Candidates {
        Candidates::Published {
            names: set(words),
            from: "a test".to_string(),
        }
    }

    /// wolf-lang `6b5db762`'s `spec/prelude.json`, byte for byte (blob
    /// ce774f97…, sha256 fb24125c…): the first published list, from s212. It
    /// is a FIXTURE, not vendored data — `vendor/upstream/` mirrors the pin,
    /// and no release carries this file yet. When a pin bump vendors the real
    /// one, these tests read the same shape from both.
    const PRELUDE_6B5DB762: &str = include_str!("../fixtures/prelude-6b5db762.json");

    /// A list less one name, for planting.
    fn without(json: &str, name: &str) -> String {
        let needle = format!("{{\"name\": \"{name}\",");
        let out: String = json
            .lines()
            .filter(|l| !l.trim_start().starts_with(&needle))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_ne!(out, json, "`{name}` was not in the list");
        out
    }

    /// The oracle at 0.2.20: every name either hand list holds resolves.
    fn every_hand_listed_name() -> Vec<&'static str> {
        TYPE_NAMES
            .iter()
            .copied()
            .chain(TYPE_POSITION_UNPAINTED.iter().map(|(n, _)| *n))
            .collect()
    }

    /// Type names wolf published AFTER `6b5db762`'s list, each classified in
    /// the hand lists at the pin that first carried it: `never` at v0.2.26
    /// (tl19). The fixture is s212's first list and stays byte for byte, so a
    /// claim about it is made against the hand lists as they would have stood
    /// then — the real lists minus these rows. The real lists against the
    /// list at the PIN are `the_vendored_list_at_the_pin_is_exactly_ours`.
    const PUBLISHED_AFTER_6B5DB762: &[&str] = &["never"];

    fn unpainted_at_6b5db762() -> Vec<(&'static str, &'static str)> {
        TYPE_POSITION_UNPAINTED
            .iter()
            .copied()
            .filter(|(n, _)| !PUBLISHED_AFTER_6B5DB762.contains(n))
            .collect()
    }

    #[test]
    fn the_first_published_list_has_twenty_five_type_names_and_they_are_ours() {
        let names = prelude_type_names(PRELUDE_6B5DB762).unwrap();
        assert_eq!(names.len(), 25, "{names:?}");
        for w in ["Scope", "Proc", "range", "List", "int", "wrapping"] {
            assert!(names.contains(w), "`{w}` missing");
        }
        // The 18 anchor words it replaces share only four with it.
        let old = set(&[
            "byte", "char", "closure", "comb", "err", "float", "fn", "generic", "interp", "list",
            "map", "method", "numlit", "range", "row", "str", "trait", "unit",
        ]);
        assert_eq!(
            names.intersection(&old).cloned().collect::<BTreeSet<_>>(),
            set(&["byte", "char", "range", "str"])
        );
        // With the list, the hand lists as they stood at that list are
        // exactly the published set.
        let names_vec = every_hand_listed_name();
        let errors = classify(
            TYPE_NAMES,
            &unpainted_at_6b5db762(),
            &BTreeSet::new(),
            &Candidates::Published {
                names,
                from: "fixture".to_string(),
            },
            &mut fake(&names_vec, &[]),
        );
        assert!(errors.is_empty(), "{errors:?}");
    }

    /// The planted red the contract asks for: the published list loses
    /// `Scope`, the compiler still resolves it, and the gate names it.
    #[test]
    fn a_planted_list_missing_scope_turns_red() {
        let planted = without(PRELUDE_6B5DB762, "Scope");
        let names = prelude_type_names(&planted).unwrap();
        assert_eq!(names.len(), 24);
        let names_vec = every_hand_listed_name();
        let errors = classify(
            TYPE_NAMES,
            &unpainted_at_6b5db762(),
            &BTreeSet::new(),
            &Candidates::Published {
                names,
                from: "planted".to_string(),
            },
            &mut fake(&names_vec, &[]),
        );
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("`Scope`") && errors[0].contains("does not name it"));
    }

    /// The vendored list at the pin, against the REAL hand lists: set
    /// equality, every name classified, no hand-listed name unpublished. At
    /// v0.2.26 that is 26 type names, `never` the one new since the fixture.
    /// A pin bump that adds a type turns this red until the name is
    /// classified — the same red `type-names-check` gives, without a binary.
    #[test]
    fn the_vendored_list_at_the_pin_is_exactly_ours() {
        let vendored = include_str!("../../vendor/upstream/spec/prelude.json");
        let names = prelude_type_names(vendored).unwrap();
        let first = prelude_type_names(PRELUDE_6B5DB762).unwrap();
        let after: BTreeSet<String> = names.difference(&first).cloned().collect();
        assert_eq!(after, set(PUBLISHED_AFTER_6B5DB762), "{names:?}");
        let names_vec = every_hand_listed_name();
        let errors = classify(
            TYPE_NAMES,
            TYPE_POSITION_UNPAINTED,
            &BTreeSet::new(),
            &Candidates::Published {
                names,
                from: "vendor/upstream/spec/prelude.json".to_string(),
            },
            &mut fake(&names_vec, &[]),
        );
        assert!(errors.is_empty(), "{errors:?}");
    }

    /// wolf-lsp#32 itself: a prelude type with no `type.*` anchor. The anchor
    /// index cannot see an unlisted `Scope`; the published list can.
    #[test]
    fn a_type_name_the_anchors_miss_is_caught_by_the_list() {
        let names = ["int", "Scope"];
        let blind = classify(
            &["int"],
            &[],
            &BTreeSet::new(),
            &anchors(&["numlit"]),
            &mut fake(&names, &[]),
        );
        assert!(blind.is_empty(), "{blind:?}");
        let seen = classify(
            &["int"],
            &[],
            &BTreeSet::new(),
            &published(&["int", "Scope"]),
            &mut fake(&names, &[]),
        );
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(seen[0].contains("`Scope`") && seen[0].contains("not classified"));
    }

    #[test]
    fn the_list_and_the_compiler_must_agree() {
        // Listed as a type, E0301 at the pin.
        let errors = classify(
            &["int"],
            &[],
            &BTreeSet::new(),
            &published(&["int", "Ghost"]),
            &mut fake(&["int"], &[]),
        );
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("`Ghost`") && errors[0].contains("disagree"));
        // `Self` is hand-listed and never published; that is not a red.
        let errors = classify(
            &["int", "Self"],
            &[],
            &BTreeSet::new(),
            &published(&["int"]),
            &mut fake(&["int", "Self"], &[]),
        );
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn the_list_reader_holds_the_schema_to_its_word() {
        assert!(
            prelude_type_names(&PRELUDE_6B5DB762.replace("wolf-prelude/0", "wolf-prelude/1"))
                .unwrap_err()
                .contains("wolf-prelude/1")
        );
        let alias = r#"{"schema":"wolf-prelude/0","names":[{"name":"int","kind":"builtin_type"},{"name":"T","kind":"alias"}]}"#;
        assert!(prelude_type_names(alias).unwrap_err().contains("`alias`"));
        // Unknown KEYS are ignored, as the schema promises.
        let extra = r#"{"schema":"wolf-prelude/0","new":1,"names":[{"name":"int","kind":"builtin_type","since":"0.2.24"}]}"#;
        assert_eq!(prelude_type_names(extra).unwrap(), set(&["int"]));
        assert!(prelude_type_names(r#"{"schema":"wolf-prelude/0","names":[]}"#).is_err());
    }

    #[test]
    fn the_source_order_is_vendored_then_compiler_then_announced_anchors() {
        let list = PRELUDE_6B5DB762;
        let no_anchors = || -> Result<BTreeSet<String>, String> { panic!("anchors read") };
        // Vendored alone (the compiler predates the command): refused — the
        // file is newer than the pin.
        assert!(
            choose_candidates(Some(list), &CompilerList::NoCommand, no_anchors)
                .unwrap_err()
                .contains("newer wolf-lang")
        );
        // Vendored and compiler agree: the vendored copy.
        let c = choose_candidates(
            Some(list),
            &CompilerList::Answered(list.to_string()),
            no_anchors,
        )
        .unwrap();
        assert!(matches!(&c, Candidates::Published { from, .. } if from.contains("vendor/")));
        // Vendored and compiler differ: red.
        assert!(
            choose_candidates(
                Some(list),
                &CompilerList::Answered(without(list, "Proc")),
                no_anchors
            )
            .unwrap_err()
            .contains("differ")
        );
        // Compiler alone.
        let c =
            choose_candidates(None, &CompilerList::Answered(list.to_string()), no_anchors).unwrap();
        assert!(matches!(&c, Candidates::Published { from, .. } if from.contains("wolf prelude")));
        // A failing command is never a reason to fall back.
        assert!(choose_candidates(None, &CompilerList::Failed("boom".into()), no_anchors).is_err());
        // Today's pin: no file, no command — the anchors, announced.
        let c = choose_candidates(None, &CompilerList::NoCommand, || Ok(set(&["range"]))).unwrap();
        assert!(
            matches!(&c, Candidates::Anchors { why, .. } if why.contains("NO PRELUDE LIST") && why.contains("wolf-lsp#32"))
        );
    }

    #[test]
    fn only_the_missing_command_reads_as_missing() {
        // 0.2.20's refusal, verbatim (~/lanes/tl17/setup.log, rc 2).
        let refusal = "wolf: `prelude` is not a wolf command.\n(`wolf --help` lists every command and shows a first program)\n";
        assert_eq!(
            compiler_list_from(false, "", refusal),
            CompilerList::NoCommand
        );
        assert_eq!(
            compiler_list_from(false, "", "wolf prelude: cannot write stdout\n"),
            CompilerList::Failed("wolf prelude: cannot write stdout".to_string())
        );
        assert_eq!(
            compiler_list_from(true, "{}", ""),
            CompilerList::Answered("{}".to_string())
        );
    }

    #[test]
    fn verdicts_read_off_stderr() {
        assert_eq!(
            verdict_from_stderr("error[E0301]: nothing named `unit` is in scope"),
            Verdict::NotAName
        );
        assert_eq!(
            verdict_from_stderr("error[E0201]: a function type spells its parameters"),
            Verdict::Syntax
        );
        assert_eq!(
            verdict_from_stderr(
                "wolf build: cannot compile this yet — a generic application left opaque"
            ),
            Verdict::Name
        );
        assert_eq!(verdict_from_stderr(""), Verdict::Name);
    }

    #[test]
    fn anchor_segments_are_the_second_word_of_type_anchors_only() {
        let json = r#"{"anchors":{"type.range.name":"10-types.md","type.byte":"10-types.md","type.byte.cast":"10-types.md","mem.region":"02-memory-model.md","gram.inv.kw":"01-grammar.md"}}"#;
        assert_eq!(type_anchor_segments(json).unwrap(), set(&["range", "byte"]));
        assert!(type_anchor_segments("{}").is_err());
    }

    #[test]
    fn the_probe_puts_the_name_in_type_position_inside_an_impl() {
        let src = probe_source("range[int]");
        assert!(src.contains("fn probe(self, x: range[int]) -> int"));
        assert!(src.starts_with("type Probe = struct {"));
    }

    #[test]
    fn a_consistent_classification_is_green() {
        let names = ["int", "byte", "range", "List"];
        let errors = classify(
            &["int", "byte"],
            &[
                ("range", "takes an argument"),
                ("List", "takes an argument"),
            ],
            &set(&["fn"]),
            &anchors(&["range", "byte", "numlit", "fn"]),
            &mut fake(&names, &["fn"]),
        );
        assert!(errors.is_empty(), "{errors:?}");
    }

    /// wolf-lsp#24's first row: `range` is anchored, resolves, and is in
    /// neither list.
    #[test]
    fn range_is_the_first_row_the_gate_catches() {
        let names = ["int", "byte", "range"];
        let errors = classify(
            &["int", "byte"],
            &[],
            &BTreeSet::new(),
            &anchors(&["range", "byte", "numlit"]),
            &mut fake(&names, &[]),
        );
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("`type.range`"));
        assert!(errors[0].contains("xtask has not classified it"));
    }

    #[test]
    fn a_removed_builtin_and_a_stale_exclusion_both_turn_red() {
        let names = ["int"];
        let errors = classify(
            &["int", "usize"],
            &[("range", "takes an argument")],
            &BTreeSet::new(),
            &anchors(&[]),
            &mut fake(&names, &[]),
        );
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("TYPE_NAMES paints `usize`"));
        assert!(errors[1].contains("stale exclusion row"));
    }

    #[test]
    fn a_reserved_keyword_among_the_anchors_is_never_asked_about() {
        // `type.fn` and `type.trait` are real anchors; even an oracle that
        // called them names must not turn them into rows.
        let names = ["int", "fn", "trait"];
        let errors = classify(
            &["int"],
            &[],
            &set(&["fn", "trait"]),
            &anchors(&["fn", "trait"]),
            &mut fake(&names, &[]),
        );
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn a_name_in_both_lists_and_a_row_without_a_reason_are_refused() {
        let names = ["int"];
        let errors = classify(
            &["int"],
            &[("int", ""), ("x", "")],
            &BTreeSet::new(),
            &anchors(&[]),
            &mut fake(&names, &[]),
        );
        assert!(errors.iter().any(|e| e.contains("both TYPE_NAMES and")));
        assert!(errors.iter().any(|e| e.contains("no reason")));
    }

    /// The vendored index really carries the first row, so the real gate is
    /// asking about `range` and not only about the two lists.
    #[test]
    fn the_vendored_anchors_carry_range_and_the_three_named_prims() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let text = std::fs::read_to_string(root.join("vendor/upstream/spec/anchors.json")).unwrap();
        let segs = type_anchor_segments(&text).unwrap();
        for w in ["range", "byte", "char", "str"] {
            assert!(segs.contains(w), "no `type.{w}` anchor at this pin");
        }
    }

    #[test]
    fn every_unpainted_row_carries_its_reason_and_none_is_also_painted() {
        for (name, reason) in TYPE_POSITION_UNPAINTED {
            assert!(!reason.trim().is_empty(), "`{name}` has no reason");
            assert!(!TYPE_NAMES.contains(name), "`{name}` is in both lists");
        }
    }
}
