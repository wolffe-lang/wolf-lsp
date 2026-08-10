//! §9 — `wolf build` and `wolf lsp` say the same thing, as a test.
//!
//! `lspconf onetruth` is the CI-facing command; this is the same check with
//! `cargo test`'s reporting, so `cargo xtask ci` exercises D34's falsifiable
//! form without anyone remembering to run a second tool.
//!
//! A divergence here is a **wolf-lang bug**, filed upstream with both records
//! attached — never normalized away, never patched around in this repo. The
//! failure message is the issue body.

mod support;

use lsp_harness::onetruth;

/// The three profiles that reach all three negotiated encodings. A divergence
/// that shows up only under one of them is an encoding bug wearing a
/// one-truth costume, which is exactly why the check runs under each.
const PROFILES: [&str; 3] = ["minimal", "utf8-first", "utf32-only"];

#[test]
fn every_sample_agrees_between_the_build_and_the_editor() {
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let samples = lsp_harness::samples(&server.root).expect("samples.toml");
    assert!(!samples.is_empty(), "no samples to check");

    let ledger = onetruth::load_ledger(&server.root).expect("divergences.toml");
    let mut filings = Vec::new();
    let mut matched: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for name in PROFILES {
        let profile = server.profile(name);
        for sample in &samples {
            let divergence =
                onetruth::check(&server.root, &server.bin, &workspace, sample, &profile)
                    .unwrap_or_else(|e| panic!("{sample} under {name}: {e}"));
            if divergence.is_empty() {
                continue;
            }
            let (known, unknown) = onetruth::triage(&divergence, &ledger);
            for (entry, _) in known {
                matched.insert(entry.id.clone());
            }
            if !unknown.is_empty() {
                filings.push(divergence.filing());
            }
        }
    }
    assert!(
        filings.is_empty(),
        "{} UNFILED divergence(s). Each is a wolf-lang bug: file it upstream with both \
         records and add it to `divergences.toml`. Never normalize it away here. The \
         text below is the issue body:\n\n{}",
        filings.len(),
        filings.join("\n---\n")
    );

    // A ledger entry that matched nothing is a bug that was fixed and a note
    // that outlived it — exactly how the same bug comes back unnoticed.
    let stale: Vec<&str> = ledger
        .iter()
        .filter(|f| !matched.contains(&f.id))
        .map(|f| f.id.as_str())
        .collect();
    assert!(
        stale.is_empty(),
        "`divergences.toml` records divergence(s) that no longer happen: {stale:?}. \
         Delete the entr(ies) and close the upstream issue(s)."
    );
}

#[test]
fn the_check_is_not_vacuous() {
    // At least one sample must actually carry a diagnostic, or the test above
    // would pass by comparing empty sets forever. The corpus pins three:
    // `fail(E0501)`, `fail(E0002)`, `fail(E0302)`.
    let Some(server) = support::server() else {
        return;
    };
    let workspace = server.samples();
    let profile = server.profile("minimal");
    let mut with_diagnostics = 0;
    for sample in lsp_harness::samples(&server.root).expect("samples.toml") {
        let bytes = std::fs::read(workspace.join(&sample)).expect("sample");
        let index = lsp_transcript::encoding::LineIndex::new(&bytes);
        let _ = (&bytes, &index, &profile);
        let claims = onetruth::build_identities(&server.bin, &workspace, &sample)
            .unwrap_or_else(|e| panic!("{sample}: {e}"));
        if !claims.is_empty() {
            with_diagnostics += 1;
        }
    }
    assert!(
        with_diagnostics >= 3,
        "only {with_diagnostics} sample(s) produce a diagnostic — the one-truth check is \
         mostly comparing empty sets, which proves very little"
    );
}
