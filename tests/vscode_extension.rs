//! The VS Code extension lane, run from `cargo test`.
//!
//! Two halves, split by what they need — the same split `tests/nvim_plugin.rs`
//! makes:
//!
//! - **Artifact checks**, which need nothing at all and therefore always run:
//!   the contribution paths resolve, the vsix excludes what it must, and the
//!   runtime dependency set is the one that ships.
//! - **The grammar lane**, which tokenizes vendored corpus samples with
//!   `vscode-textmate` and `vscode-oniguruma` — the exact tokenizer and exact
//!   regex engine VS Code runs — and compares reviewed scope snapshots. It
//!   needs `node` and an installed `node_modules`, and **skips loudly** without
//!   either, exactly the way the server-dependent suites skip without a `wolf`
//!   binary (ls00 §3).
//!
//! **What this lane deliberately does NOT run: headless VS Code.**
//! `@vscode/test-electron` *downloads a VS Code build* on first use — roughly
//! 150 MB over the network — and a `cargo test` on a developer's laptop that
//! silently starts a download is a `cargo test` people stop running. That lane
//! is a separate CI job (`vscode extension`), the same way the Neovim plugin
//! lane is a separate job from `test`, and for a sharper version of the same
//! reason: there, the editor might be absent; here, running it would *install*
//! one.
//!
//! Note what neither half needs: a `wolf` binary. The grammar is a property of
//! the pinned EBNF, and the contributions are properties of files in the tree.

use std::path::{Path, PathBuf};
use std::process::Command;

fn ext_root() -> PathBuf {
    lsp_harness::repo_root().join("clients").join("vscode")
}

fn manifest() -> serde_json::Value {
    let path = ext_root().join("package.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", lsp_harness::slash_path(&path)));
    serde_json::from_str(&text).expect("clients/vscode/package.json is valid JSON")
}

// ============================================================ artifacts ====

/// Every file `package.json` promises a user is actually there.
///
/// This is the check a broken `.vscodeignore` fails. A packaged extension whose
/// `grammars` entry points at a path the vsix excluded installs cleanly,
/// activates cleanly, and highlights nothing — the failure mode with the
/// longest gap between cause and symptom.
#[test]
fn every_contributed_path_exists() {
    let pkg = manifest();
    let root = ext_root();
    let mut checked = 0;

    for grammar in pkg["contributes"]["grammars"]
        .as_array()
        .expect("contributes.grammars")
    {
        let rel = grammar["path"].as_str().expect("a grammar path");
        let path = root.join(rel.trim_start_matches("./"));
        assert!(
            path.is_file(),
            "contributes.grammars names `{rel}`, which does not exist — \
             run `cargo xtask grammar-generate`"
        );
        checked += 1;
    }

    for language in pkg["contributes"]["languages"]
        .as_array()
        .expect("contributes.languages")
    {
        if let Some(rel) = language["configuration"].as_str() {
            let path = root.join(rel.trim_start_matches("./"));
            assert!(
                path.is_file(),
                "a language contribution names `{rel}`, which does not exist"
            );
            checked += 1;
        }
    }

    assert!(checked >= 4, "expected at least four contributed paths");
}

/// The three languages the sprint names, mapped the way the extension means.
///
/// `.wolfi` having its own id is the deliberate narrowing recorded in
/// `clients/vscode/README.md`: `wolf lsp` has no `.wolfi` path at this pin (the
/// format is binary), so it gets highlighting and no client. Asserting it here
/// means the narrowing cannot be undone by accident — only on purpose, by
/// someone who also updates this test and the README.
#[test]
fn the_language_contributions_are_the_documented_three() {
    let pkg = manifest();
    let languages = pkg["contributes"]["languages"]
        .as_array()
        .expect("contributes.languages");

    let id_for_extension = |want: &str| -> Option<String> {
        languages.iter().find_map(|l| {
            l["extensions"]
                .as_array()?
                .iter()
                .any(|e| e.as_str() == Some(want))
                .then(|| l["id"].as_str().unwrap_or_default().to_string())
        })
    };

    assert_eq!(id_for_extension(".lu").as_deref(), Some("wolf"));
    assert_eq!(
        id_for_extension(".wolfi").as_deref(),
        Some("wolfi"),
        "`.wolfi` must not share the `wolf` language id — that would put it under \
         the client's documentSelector, and the server has no `.wolfi` path at this pin"
    );

    let manifest_lang = languages
        .iter()
        .find(|l| {
            l["filenames"]
                .as_array()
                .is_some_and(|f| f.iter().any(|n| n.as_str() == Some("wolf.pkg")))
        })
        .expect("a language claiming wolf.pkg");
    assert_eq!(manifest_lang["id"].as_str(), Some("wolf-pkg"));
    assert!(
        manifest_lang["filenames"]
            .as_array()
            .is_some_and(|f| f.iter().any(|n| n.as_str() == Some("wolf.sum"))),
        "wolf.sum shares the manifest language"
    );
}

/// The vsix ships the extension, not the repository.
///
/// ls05 §3: "a vsix that ships the corpus is a vsix nobody downloads". The
/// corpus is not in this directory at all, which is the structural half; this
/// is the half that catches the test tree and the sources.
#[test]
fn the_vscodeignore_excludes_the_sources_and_the_test_tree() {
    let path = ext_root().join(".vscodeignore");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", lsp_harness::slash_path(&path)));
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    for required in ["src/**", "out/test/**", ".vscode-test/**"] {
        assert!(
            lines.contains(&required),
            ".vscodeignore does not exclude `{required}` — that is either a vsix \
             carrying a full VS Code download, or one carrying its own tests"
        );
    }
}

/// One runtime dependency, and it is the client library.
///
/// `dependencies` is exactly what `vsce` bundles into the vsix, so this is a
/// check on what reaches a user's machine rather than on tidiness. ls00 §7's
/// dependency thinness applies to the editor layer too — and the extension has
/// no work of its own that could justify a second one.
#[test]
fn the_extension_ships_one_runtime_dependency() {
    let pkg = manifest();
    let deps: Vec<String> = pkg["dependencies"]
        .as_object()
        .expect("dependencies")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        deps,
        vec!["vscode-languageclient".to_string()],
        "a new runtime dependency ships to every user; a dev one does not"
    );
}

// ========================================================= grammar lane ====

/// `node`, or `None` with the reason printed.
fn node() -> Option<String> {
    match Command::new("node").arg("--version").output() {
        Ok(out) if out.status.success() => {
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        _ => {
            println!(
                "SKIP: no working `node` on PATH — the grammar lane needs one. The \
                 grammar itself is unaffected; `cargo xtask grammar-drift` still \
                 proves it matches the pin, and `clients/vscode/` is data until a \
                 tokenizer reads it."
            );
            None
        }
    }
}

fn npm(root: &Path, args: &[&str]) -> std::process::Output {
    // `npm.cmd` on Windows: npm ships as a shell script plus a batch wrapper,
    // and `CreateProcess` will not run the extensionless script.
    let program = if cfg!(windows) { "npm.cmd" } else { "npm" };
    Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .unwrap_or_else(|e| panic!("spawn {program} {args:?}: {e}"))
}

/// The reviewed scope snapshots still hold, under the tokenizer VS Code runs.
#[test]
fn the_grammar_tokenizes_the_corpus_as_reviewed() {
    let Some(version) = node() else {
        return;
    };
    let root = ext_root();

    if !root.join("node_modules").is_dir() {
        println!(
            "SKIP: clients/vscode/node_modules is absent — run `npm ci` in \
             clients/vscode first. This lane does not install anything on its \
             own: a `cargo test` that starts a package download is a `cargo \
             test` people stop running."
        );
        return;
    }
    println!("node: {version}");

    let compile = npm(&root, &["run", "--silent", "compile"]);
    assert!(
        compile.status.success(),
        "the extension does not compile\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr),
    );

    let out = npm(&root, &["run", "--silent", "test:grammar"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "the grammar lane failed\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    // A runner that found no cases exits 0 with nothing to say, which is the
    // silent-green failure this repository is built to refuse.
    assert!(
        stdout.contains(" passed, 0 failed"),
        "the runner produced no summary — did it find any cases?\n{stdout}"
    );
    println!("{stdout}");
}
