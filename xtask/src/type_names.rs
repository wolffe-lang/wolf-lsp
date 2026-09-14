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
//! # What the gate cannot see
//!
//! - A builtin added upstream WITHOUT a two-segment `type.<name>` anchor of
//!   its own spelling — a `u128` documented under `[type.numlit]`, say, or a
//!   container whose anchor is lower-case while its name is not (`Map` under
//!   `type.map`). The candidate set is the spec's index, and the index is
//!   only as complete as the spec's anchoring discipline. The removal half
//!   still holds for such a name once a human has listed it.
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

/// The classification, with the oracle abstracted so a test can fake it.
///
/// `reserved` is `reserved_kw` from the vendored EBNF: a keyword can appear
/// as an anchor segment (`type.fn`, `type.trait`) and can never be a type
/// NAME, so it is dropped before the compiler is asked.
pub(crate) fn classify(
    type_names: &[&str],
    unpainted: &[(&str, &str)],
    reserved: &BTreeSet<String>,
    anchor_segments: &BTreeSet<String>,
    verdict: &mut dyn FnMut(&str) -> Verdict,
) -> Vec<String> {
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
    candidates.extend(
        anchor_segments
            .iter()
            .map(String::as_str)
            .filter(|s| !reserved.contains(*s)),
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
    for seg in anchor_segments {
        let seg = seg.as_str();
        if reserved.contains(seg) || painted.contains(seg) || excluded.contains(seg) {
            continue;
        }
        if verdicts[seg] == Verdict::Name {
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
    let anchors = match std::fs::read_to_string(vendor.join("anchors.json"))
        .map_err(|e| e.to_string())
        .and_then(|t| type_anchor_segments(&t))
    {
        Ok(a) => a,
        Err(e) => return Health::Ok(vec![format!("vendor/upstream/spec/anchors.json: {e}")]),
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
            &anchors,
            &mut verdict,
        );
        errors.extend(found);
    }
    let _ = std::fs::remove_dir_all(&scratch);
    if errors.is_empty() {
        eprintln!(
            "type-names: {} painted, {} unpainted, {} anchor candidates, all classified against {}",
            TYPE_NAMES.len(),
            TYPE_POSITION_UNPAINTED.len(),
            anchors.len(),
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
            &set(&["range", "byte", "numlit", "fn"]),
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
            &set(&["range", "byte", "numlit"]),
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
            &BTreeSet::new(),
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
            &set(&["fn", "trait"]),
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
            &BTreeSet::new(),
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
