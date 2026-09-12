//! The builtin TYPE list, and the four editor artifacts that copy it.
//!
//! # The hole this closes (wolf-lsp#21)
//!
//! The builtin type names live in **five** places, and until this module
//! exactly **one** of them was gated:
//!
//! | place | what gated it before |
//! |---|---|
//! | [`crate::vscode::TYPE_NAMES`] | `grammar-drift`, byte-comparing the derived `.tmLanguage.json` |
//! | `clients/nvim/syntax/wolf.vim` | nothing — `nvim-check` reads only between the `reserved-kw-*` markers, and the type row sits outside them |
//! | `clients/emacs/wolf-mode.el` | nothing — `emacs-check` is the same shape for the elisp keyword block |
//! | `clients/zed/languages/wolf/highlights.scm` | nothing — `config-check` asserted only that the file contained *some* pattern |
//! | `clients/emacs/README.md` | transitively, as verbatim containment of the `.el` — so it drifts *with* it, never *from* it |
//!
//! The five are not all expected to carry the *same* list: see
//! [`expected_for_queries`] for the one deliberate difference, which is
//! le06's and which #21's own suggested shape would have erased.
//!
//! A type added upstream could therefore be added to one, three or none of
//! them and CI stayed green either way. It is the failure `docs/MATRIX.md`
//! was built to prevent, one layer down — and it had already happened: zed's
//! copy carried `usize` and `isize`, which wolf does not have, and lacked
//! `wrapping` and `Self`, which it does. That is le06's correction in
//! tree-sitter-wolf ("unexamined Rust-isms: no `spec/*.md` names either one,
//! and neither is in the compiler's closed builtin set"), never applied here.
//!
//! # Why [`crate::vscode::TYPE_NAMES`] is the source
//!
//! 1. It is the only one of the five that was **already gated**, so making it
//!    the source adds no new unguarded thing.
//! 2. It is the only one written in **Rust**, inside the xtask CI already
//!    runs. The other four are TextMate JSON, Vimscript, elisp and a
//!    tree-sitter query; something has to be the list the checker can simply
//!    read instead of parse.
//! 3. Its doc comment is where the **reasoning** already lives — the ruling
//!    that `range` does not join (tl04/#16), le06's `usize`/`isize`
//!    correction, and the argument that these four artifacts are REGULAR and
//!    so cannot paint a contextual name. The list stays next to the argument
//!    for the list.
//!
//! # What this does NOT cover
//!
//! Nothing here checks `TYPE_NAMES` against **wolfc's actual
//! `BUILTIN_TYPES`**. `vendor/upstream/` carries `spec/grammar.ebnf` and
//! `PIN` and no `prelude.rs`, and the type set is not derivable from the
//! EBNF — `clients/nvim/syntax/wolf.vim` says so in its own comment ("Not
//! derivable from the EBNF — re-read wolf_sema/src/prelude.rs at each pin").
//! So this module makes the five copies agree with each other; it cannot yet
//! make them agree with the compiler. Filed separately.

use std::collections::BTreeSet;

/// The builtin type set, as the source spells it: the 17 prims plus `Self`.
///
/// The expectation for the three **REGULAR** consumers — VS Code's TextMate
/// grammar, nvim's `syn keyword` row, emacs' font-lock list. They match bare
/// words with no structure available to them, so `Self` has to be in the list
/// or it does not paint at all.
pub fn expected() -> BTreeSet<String> {
    crate::vscode::TYPE_NAMES
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// The same set **without `Self`** — the expectation for a tree-sitter query.
///
/// # Why this is not just [`expected`]
///
/// wolf-lsp#21's suggested shape was plain set-equality against the source for
/// nvim, emacs *and* zed alike. That is right for the first two and wrong for
/// zed, and following it would have re-broken a ruling le06 had already made
/// in tree-sitter-wolf, whose `queries/highlights.scm` says it in as many
/// words:
///
/// > `Self` stays out deliberately: it is not a builtin type NAME but a
/// > context-bound alias, and it already paints through `(type_path (path
/// > (identifier)) @type)` wherever it can appear.
///
/// zed's `highlights.scm` is the same artifact class as that file — a
/// tree-sitter query over the same grammar — so it inherits the same ruling.
/// A structural consumer knows *where a token stands*, so it paints `Self`
/// where `Self` can legally appear instead of wherever the six letters occur.
/// Putting `Self` in the `#any-of?` list would paint the word in expression
/// and binding position too, which is the REGULAR consumers' compromise and
/// not something a query has to accept.
///
/// The exclusion is therefore load-bearing, and [`query_paints_self_structurally`]
/// is what keeps it honest: `Self` may be left out of the any-of list only
/// while the `type_path` rule that actually paints it is still there.
pub fn expected_for_queries() -> BTreeSet<String> {
    let mut set = expected();
    set.remove("Self");
    set
}

/// A tree-sitter query that omits `Self` must still paint it structurally.
///
/// Without this, "zed is allowed to leave `Self` out" decays into "zed does
/// not paint `Self`", which is the drift this module exists to catch, arrived
/// at by a different road.
///
/// Matched as a TOP-LEVEL form, never as a substring. The substring spelling
/// was written first and a planted break proved it could not go red:
/// `(type_path (path (identifier) @type))` also occurs *nested* inside
/// `(struct_expression name: (type_path (path (identifier) @type)))`, so a
/// `contains` was satisfied by a rule that paints struct-literal names and
/// says nothing about `Self` in type position. Query forms nest; a substring
/// test over a query file is blind by construction.
pub fn query_paints_self_structurally(scm: &str) -> bool {
    scm.lines()
        .any(|l| l.trim() == "(type_path (path (identifier) @type))")
}

/// Set-equality against an expectation, **both directions**.
///
/// Both directions on purpose, and this is the whole point of the module: a
/// containment check in one direction only is what let zed's copy carry two
/// names wolf does not have for as long as it did.
pub fn compare_with(
    label: &str,
    expected: &BTreeSet<String>,
    found: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    for missing in expected.difference(found) {
        errors.push(format!(
            "{label} is missing the builtin type `{missing}` — a type the compiler knows and \
             the editor does not renders as a plain identifier"
        ));
    }
    for extra in found.difference(expected) {
        errors.push(format!(
            "{label} paints `{extra}` as a builtin type, and the builtin set does not contain \
             it — an invented type teaches a language that does not exist"
        ));
    }
}

/// [`compare_with`] against [`expected`] — for the REGULAR consumers.
pub fn compare(label: &str, found: &BTreeSet<String>, errors: &mut Vec<String>) {
    compare_with(label, &expected(), found, errors);
}

/// [`compare_with`] against [`expected_for_queries`] — for tree-sitter queries.
pub fn compare_query(label: &str, found: &BTreeSet<String>, errors: &mut Vec<String>) {
    compare_with(label, &expected_for_queries(), found, errors);
}

/// Every `"…"`-delimited token in `text`, in order.
///
/// Shared by the elisp and tree-sitter-query readers, which both quote with
/// `"`. Deliberately NOT `main.rs`'s `quoted`, which reads the EBNF's
/// SINGLE-quoted terminals.
fn double_quoted(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for c in text.chars() {
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

/// The balanced `(`…`)` slice starting at the `(` at or after `from`.
///
/// Quote-aware, so a `)` inside a string does not close the form. Returns the
/// inside of the parens.
fn balanced(text: &str, from: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let open = from + text[from..].find('(')?;
    let mut depth = 0usize;
    let mut in_str = false;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'"' => in_str = !in_str,
            b'(' if !in_str => depth += 1,
            b')' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[open + 1..i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// `syn keyword wolfType bool byte char …` → the set of names.
///
/// `None` when the row is absent, which is a failure and not an empty set — a
/// check that silently passes when its input disappears is worse than no
/// check. The same sentence `elisp_keywords` and `syntax_keywords` make.
pub fn vim_types(vim: &str) -> Option<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    let mut saw = false;
    for line in vim.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("syn keyword wolfType") else {
            continue;
        };
        saw = true;
        for token in rest.split_whitespace() {
            // `syn keyword` accepts trailing arguments like `contained`; the
            // type row uses none, and one appearing is drift worth a red.
            out.insert(token.to_string());
        }
    }
    saw.then_some(out)
}

/// `(defconst wolf-mode-builtin-types '("Self" "bool" …) "docstring")` → the set.
///
/// Reads only the quoted list, never the docstring that follows it — hence
/// [`balanced`] rather than a line scan; the docstring is full of `"`.
pub fn elisp_types(el: &str) -> Option<BTreeSet<String>> {
    let at = el.find("(defconst wolf-mode-builtin-types")?;
    // Skip the defconst's own `(`, then take the quoted list after the name.
    let after_name = at + "(defconst wolf-mode-builtin-types".len();
    let list = balanced(el, after_name)?;
    Some(double_quoted(list).into_iter().collect())
}

/// zed's `((identifier) @type.builtin (#any-of? @type.builtin "int" …))` → the set.
pub fn scm_types(scm: &str) -> Option<BTreeSet<String>> {
    let at = scm.find("#any-of? @type.builtin")?;
    // `balanced` from the start of the `(#any-of? …)` form.
    let open = scm[..at].rfind('(')?;
    let list = balanced(scm, open)?;
    let mut out: BTreeSet<String> = double_quoted(list).into_iter().collect();
    // The predicate's first argument is the capture name, not a type.
    out.remove("@type.builtin");
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vim_row_reads_the_names() {
        let got = vim_types("\" a comment\nsyn keyword wolfType bool byte Self\n").unwrap();
        assert_eq!(
            got,
            ["bool", "byte", "Self"]
                .iter()
                .map(|s| s.to_string())
                .collect()
        );
    }

    #[test]
    fn vim_row_absent_is_none_not_empty() {
        assert!(vim_types("syn keyword wolfKeyword fn let\n").is_none());
    }

    #[test]
    fn elisp_list_stops_before_the_docstring() {
        let el = "(defconst wolf-mode-builtin-types\n  '(\"Self\" \"bool\")\n  \"A docstring \
                  naming \\\"int\\\" in prose.\")\n";
        let got = elisp_types(el).unwrap();
        assert_eq!(
            got,
            ["Self", "bool"].iter().map(|s| s.to_string()).collect()
        );
    }

    #[test]
    fn elisp_absent_is_none() {
        assert!(elisp_types("(defconst wolf-mode-other '(\"x\"))").is_none());
    }

    #[test]
    fn scm_predicate_drops_the_capture_name() {
        let scm = "((identifier) @type.builtin\n  (#any-of? @type.builtin\n    \"int\" \
                   \"bool\"))\n";
        let got = scm_types(scm).unwrap();
        assert_eq!(got, ["int", "bool"].iter().map(|s| s.to_string()).collect());
    }

    #[test]
    fn scm_absent_is_none() {
        assert!(scm_types("((identifier) @variable)").is_none());
    }

    /// The gate must be able to SEE a divergence, in both directions. This is
    /// zed's actual pre-fix list, which had two names too many and (as a
    /// query) one too few; if this test ever stops producing errors the gate
    /// has gone blind in the exact way #21 describes.
    #[test]
    fn compare_catches_both_directions() {
        let zed_before: BTreeSet<String> = [
            "int", "uint", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
            "bool", "str", "byte", "usize", "isize", "char",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        // As a REGULAR consumer would be judged: `Self` is owed too.
        let mut errors = Vec::new();
        compare("zed", &zed_before, &mut errors);
        assert_eq!(errors.len(), 4, "{errors:#?}");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("missing the builtin type `Self`"))
        );

        // As a QUERY is judged: three, and `Self` is NOT owed. The difference
        // between these two numbers is le06's ruling, and #21's suggested
        // shape would have erased it.
        let mut errors = Vec::new();
        compare_query("zed", &zed_before, &mut errors);
        assert_eq!(errors.len(), 3, "{errors:#?}");
        assert!(!errors.iter().any(|e| e.contains("`Self`")));
        assert!(
            errors
                .iter()
                .any(|e| e.contains("missing the builtin type `wrapping`"))
        );
        assert!(errors.iter().any(|e| e.contains("paints `usize`")));
        assert!(errors.iter().any(|e| e.contains("paints `isize`")));
    }

    /// The substring spelling of the `Self` check was GREEN with the rule
    /// deleted, because the same text nests inside the struct-literal rule.
    /// Found by a planted break; kept as a test so it cannot come back.
    #[test]
    fn nested_type_path_does_not_satisfy_the_self_check() {
        let nested_only = "(struct_expression name: (type_path (path (identifier) @type)))\n";
        // The trap, stated: a `contains` is satisfied by this file.
        assert!(nested_only.contains("(type_path (path (identifier) @type))"));
        assert!(!query_paints_self_structurally(nested_only));
        let real = format!("{nested_only}(type_path (path (identifier) @type))\n");
        assert!(query_paints_self_structurally(&real));
    }

    #[test]
    fn the_query_expectation_is_the_source_minus_self() {
        assert_eq!(expected().len(), 18);
        assert_eq!(expected_for_queries().len(), 17);
        assert!(expected().contains("Self"));
        assert!(!expected_for_queries().contains("Self"));
    }

    /// The committed artifacts all agree with the source. This is the gate
    /// itself, at unit-test range.
    #[test]
    fn the_four_consumers_agree_with_the_source() {
        let root = crate::repo_root();
        let mut errors = Vec::new();
        let vim = std::fs::read_to_string(root.join("clients/nvim/syntax/wolf.vim")).unwrap();
        compare("nvim", &vim_types(&vim).unwrap(), &mut errors);
        let el = std::fs::read_to_string(root.join("clients/emacs/wolf-mode.el")).unwrap();
        compare("emacs", &elisp_types(&el).unwrap(), &mut errors);
        let scm = std::fs::read_to_string(root.join("clients/zed/languages/wolf/highlights.scm"))
            .unwrap();
        compare_query("zed", &scm_types(&scm).unwrap(), &mut errors);
        assert!(
            query_paints_self_structurally(&scm),
            "zed must paint Self structurally"
        );
        assert!(errors.is_empty(), "{errors:#?}");
    }
}
