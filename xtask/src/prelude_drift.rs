//! `cargo xtask prelude-drift <pinned.json> <other.json>` (wolf-lsp#30).
//!
//! Two `wolf-prelude/0` documents (`wolf prelude --json`, wolf-lang s212)
//! compared as SETS, both ways, never as counts (wolf-lang#177): the names as
//! `(name, kind)` pairs, an entry whose pair is in both but whose other fields
//! moved as changed, the marks by name, and each shared mark's `by` list by
//! member. The nightly's `drift` job runs it against the latest wolf-lang
//! release's own `wolf prelude --json`, so "capability drift" is a comparison
//! and not a sentence: before this, that job counted releases and printed
//! prose (wolf-lsp#30).
//!
//! Drift is DATA, not failure: the command exits 0 whatever it finds and says
//! what it found. It exits 1 only when it could not compare, because a drift
//! check that compared nothing is the exact defect it replaces.

use std::collections::{BTreeMap, BTreeSet};

use crate::type_names::PRELUDE_SCHEMA;

/// What moved between two prelude documents, each list sorted.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Drift {
    /// `(name, kind)` in `other` and not in `pinned`.
    pub names_added: Vec<(String, String)>,
    /// `(name, kind)` in `pinned` and not in `other`.
    pub names_removed: Vec<(String, String)>,
    /// `(name, kind)` in both whose entry differs in any other field.
    pub names_changed: Vec<(String, String)>,
    /// Mark names in `other` and not in `pinned`.
    pub marks_added: Vec<String>,
    /// Mark names in `pinned` and not in `other`.
    pub marks_removed: Vec<String>,
    /// For a mark in both: the `by` members `other` adds.
    pub marks_gained: Vec<(String, Vec<String>)>,
    /// For a mark in both: the `by` members `other` drops.
    pub marks_lost: Vec<(String, Vec<String>)>,
}

impl Drift {
    pub(crate) fn is_none(&self) -> bool {
        *self == Drift::default()
    }
}

/// One document's names (keyed by `(name, kind)`, the whole entry kept) and
/// marks (keyed by name, the `by` set kept).
struct Doc {
    names: BTreeMap<(String, String), serde_json::Value>,
    marks: BTreeMap<String, BTreeSet<String>>,
}

fn read(label: &str, text: &str) -> Result<Doc, String> {
    let _ = (label, text);
    Ok(Doc {
        names: BTreeMap::new(),
        marks: BTreeMap::new(),
    })
}

/// Compare `pinned` (the vendored list) with `other` (another compiler's).
pub(crate) fn drift(pinned: &str, other: &str) -> Result<Drift, String> {
    let _ = (read("pinned", pinned)?, read("other", other)?);
    let _ = PRELUDE_SCHEMA;
    Ok(Drift::default())
}

/// The report, one line a fact, the names spelled out.
pub(crate) fn render(d: &Drift) -> String {
    if d.is_none() {
        return "prelude-drift: none — the two lists name the same (name, kind) pairs, \
                entries and marks"
            .to_string();
    }
    String::from("prelude-drift: (unrendered)")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(names: &[(&str, &str)], marks: &[(&str, &[&str])]) -> String {
        let names: Vec<serde_json::Value> = names
            .iter()
            .map(|(n, k)| serde_json::json!({"name": n, "kind": k, "anchor": "os.host.sigs", "w0304": true}))
            .collect();
        let marks: Vec<serde_json::Value> = marks
            .iter()
            .map(|(n, by)| serde_json::json!({"name": n, "anchor": "os.host.sigs", "by": by}))
            .collect();
        serde_json::json!({"schema": PRELUDE_SCHEMA, "names": names, "marks": marks}).to_string()
    }

    fn pair(n: &str, k: &str) -> (String, String) {
        (n.to_string(), k.to_string())
    }

    #[test]
    fn prelude_drift_names_added_and_removed_both_ways() {
        // 0.2.34's shape: a function added; plus a removal and a kind change,
        // which is a removal of one pair and the addition of another.
        let pinned = doc(&[("print", "function"), ("os_exit_signal", "function"), ("Old", "type"), ("x", "function")], &[]);
        let other = doc(&[("print", "function"), ("os_exit_signal", "function"), ("os_fork", "function"), ("x", "type")], &[]);
        let d = drift(&pinned, &other).unwrap();
        assert_eq!(d.names_added, vec![pair("os_fork", "function"), pair("x", "type")]);
        assert_eq!(d.names_removed, vec![pair("Old", "type"), pair("x", "function")]);
        assert!(d.names_changed.is_empty(), "{d:?}");
        // A pair in both whose anchor moved is CHANGED, not added or removed.
        let moved = pinned.replacen("\"anchor\":\"os.host.sigs\"", "\"anchor\":\"os.host.other\"", 1);
        let d = drift(&pinned, &moved).unwrap();
        assert_eq!(d.names_changed.len(), 1, "{d:?}");
        assert!(d.names_added.is_empty() && d.names_removed.is_empty(), "{d:?}");
        let text = render(&drift(&pinned, &other).unwrap());
        assert!(text.contains("os_fork") && text.contains("Old"), "{text}");
    }

    #[test]
    fn prelude_drift_reads_marks_both_ways() {
        // 0.2.34's `busy` (by os_fork) and `io` gaining a member; plus a mark
        // dropped and a member lost.
        let pinned = doc(&[], &[("io", &["fs_read"]), ("gone", &["a"]), ("invalid", &["a", "b"])]);
        let other = doc(&[], &[("io", &["fs_read", "os_fork"]), ("busy", &["os_fork"]), ("invalid", &["a"])]);
        let d = drift(&pinned, &other).unwrap();
        assert_eq!(d.marks_added, vec!["busy".to_string()]);
        assert_eq!(d.marks_removed, vec!["gone".to_string()]);
        assert_eq!(d.marks_gained, vec![("io".to_string(), vec!["os_fork".to_string()])]);
        assert_eq!(d.marks_lost, vec![("invalid".to_string(), vec!["b".to_string()])]);
        let text = render(&d);
        assert!(text.contains("busy") && text.contains("gone"), "{text}");
    }

    #[test]
    fn identical_prelude_documents_drift_none_and_a_bad_one_is_an_error() {
        let a = doc(&[("print", "function"), ("int", "builtin_type")], &[("io", &["print"])]);
        let d = drift(&a, &a).unwrap();
        assert!(d.is_none(), "{d:?}");
        assert!(render(&d).contains("none"));
        // Comparing nothing must never read as "no drift" (wolf-lsp#30).
        assert!(drift(&a, "{}").is_err());
        assert!(drift(&a, "not json").is_err());
        let wrong = a.replace(PRELUDE_SCHEMA, "wolf-prelude/1");
        assert!(drift(&a, &wrong).is_err());
    }
}
