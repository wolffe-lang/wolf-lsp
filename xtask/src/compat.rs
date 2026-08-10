//! ls07 §3 — the compatibility statement, and the gate that keeps it earned.
//!
//! **PUBLISHES NOTHING.** Every command in this file reads the repository and
//! writes files inside it. Nothing here contacts a marketplace, a registry or a
//! remote; the steps that would are human-gated and listed in
//! `docs/DISTRIBUTION.md`.
//!
//! Each shipping client carries a `compat.json` declaring the range of `wolf`
//! it supports. The sprint's rule is that the range is **earned**: `max_tested`
//! may only name a version the conformance suite has actually been run against,
//! and the release path refuses a declared range wider than the evidence.
//!
//! Today the evidence set has exactly one member. `vendor/upstream/PIN` names
//! one commit and one `wolf --version` string, every transcript in this repo
//! was recorded against it, and no second wolf version exists anywhere to test
//! against — wolf-lang tags no releases. So the schema below supports ranges
//! and the data claims a point. Writing `max_tested = "0.1.0"` today turns
//! `cargo xtask compat-check` red, which is the whole design: the file that
//! would lie is the file the gate reads.
//!
//! Two artifacts are GENERATED from the same data, for the two clients that can
//! run code at startup:
//!
//! * `clients/nvim/lua/wolf/compat.lua`  — read by `:checkhealth wolf`
//! * `clients/vscode/src/compat.ts`      — read by the activation notification
//!
//! and one documentation table, `docs/COMPAT.md` between its markers. A
//! hand-edit of any of the three is drift and fails the check, for the same
//! reason `pin.lua` and `pin.ts` are generated: a hand-maintained copy of the
//! pin goes stale on the first bump, silently, in the one place whose job is
//! noticing staleness.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{pin_value, slash};

/// The clients that ship an installable artifact, and therefore owe a range.
///
/// `fackr`, `facsimile` and `jetbrains` are deliberately absent and the reason
/// is structural rather than an oversight: none of them is an artifact this
/// repository versions or cuts. fackr's and facsimile's deliverable is a patch
/// series applied to somebody else's editor — the range statement travels with
/// the patch, into their repository, under their version scheme — and
/// `jetbrains/` is a page of prose with no artefact at all. A `compat.json` in
/// any of the three would declare a compatibility range for a release that does
/// not exist.
pub const SHIPPING_CLIENTS: &[&str] = &["emacs", "helix", "nvim", "vscode", "zed"];

/// One client's declared range, after parsing and before validation.
struct Compat {
    dir: String,
    client: String,
    client_version: String,
    artifact: String,
    min: String,
    max_tested: String,
    pin: String,
    pin_version_string: String,
    verified_date: String,
    verified_host: String,
    evidence: Vec<String>,
    caveat: Option<String>,
    warning_surface: String,
}

/// `compat-check` (`write == false`) and `compat-generate` (`write == true`).
pub fn check(root: &Path, write: bool) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();

    let pin_path = root.join("vendor").join("upstream").join("PIN");
    let pin_text = match std::fs::read_to_string(&pin_path) {
        Ok(t) => t,
        Err(e) => return vec![format!("{}: {e}", slash(&pin_path))],
    };
    let pin_commit = pin_value(&pin_text, "commit");
    let pin_version = pin_value(&pin_text, "version");

    // The earned set: every wolf version this repository holds evidence for.
    //
    // It is derived, not listed, and it is a set rather than a scalar so the
    // shape survives the day a second version enters the tree. Today the
    // derivation has one input, because the repo pins one commit and every
    // transcript under `transcripts/` was recorded against it.
    let earned = match earned_versions(&pin_version) {
        Ok(set) => set,
        Err(e) => return vec![e],
    };

    let mut parsed: BTreeMap<String, Compat> = BTreeMap::new();
    for dir in SHIPPING_CLIENTS {
        let path = root.join("clients").join(dir).join("compat.json");
        match std::fs::read_to_string(&path) {
            Ok(text) => match parse(dir, &text) {
                Ok(c) => {
                    parsed.insert((*dir).to_string(), c);
                }
                Err(e) => errors.push(format!("{}: {e}", slash(&path))),
            },
            Err(e) => errors.push(format!(
                "{}: {e} — every shipping client declares a wolf range (ls07 §3)",
                slash(&path)
            )),
        }
    }

    // A `compat.json` under a client that ships nothing is a range for a release
    // that does not exist. Caught here rather than ignored, because the next
    // person to add one would otherwise get silence.
    for entry in std::fs::read_dir(root.join("clients"))
        .into_iter()
        .flatten()
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if !entry.path().is_dir() || SHIPPING_CLIENTS.contains(&name.as_str()) {
            continue;
        }
        if entry.path().join("compat.json").is_file() {
            errors.push(format!(
                "clients/{name}/compat.json exists, but `{name}` ships no artifact this repo \
                 versions — either add it to SHIPPING_CLIENTS with its release path, or delete \
                 the file. A range for a release nobody cuts is a claim nobody can check"
            ));
        }
    }

    for (dir, c) in &parsed {
        let at = format!("clients/{dir}/compat.json");

        if c.pin != pin_commit {
            errors.push(format!(
                "{at}: `wolf.pin` is {} but vendor/upstream/PIN says {pin_commit} \
                 — a pin bump moves both, in one commit (docs/RELEASE.md step 1)",
                c.pin
            ));
        }
        if c.pin_version_string != pin_version {
            errors.push(format!(
                "{at}: `wolf.pin_version_string` is {:?} but PIN says {pin_version:?}",
                c.pin_version_string
            ));
        }

        // THE GATE. The whole sprint target in one comparison.
        if !earned.contains(&c.max_tested) {
            errors.push(format!(
                "{at}: `wolf.max_tested` is {:?}, which the suite has never been run against. \
                 Earned versions today: {}. `max_tested` may only name a version a recorded \
                 session or a live suite run proves (ls07 §3) — widening it here would make \
                 every install instruction downstream a guess",
                c.max_tested,
                earned.join(", ")
            ));
        }
        match (triple(&c.min), triple(&c.max_tested)) {
            (Some(lo), Some(hi)) => {
                if lo > hi {
                    errors.push(format!(
                        "{at}: `wolf.min` {} is above `wolf.max_tested` {} — an empty range",
                        c.min, c.max_tested
                    ));
                }
            }
            _ => errors.push(format!(
                "{at}: `wolf.min` / `wolf.max_tested` must be MAJOR.MINOR.PATCH, got {:?} / {:?}",
                c.min, c.max_tested
            )),
        }

        if c.evidence.is_empty() {
            errors.push(format!(
                "{at}: `verified.evidence` is empty — a range with no evidence is the shape \
                 this file exists to prevent"
            ));
        }
        for item in &c.evidence {
            // Repo-relative paths are checked; command lines and prose are not.
            let looks_like_path = item.contains('/') && !item.contains(' ');
            if looks_like_path && !root.join(item).exists() {
                errors.push(format!(
                    "{at}: `verified.evidence` names `{item}`, which does not exist"
                ));
            }
        }
        if !is_iso_date(&c.verified_date) {
            errors.push(format!(
                "{at}: `verified.date` must be YYYY-MM-DD, got {:?}",
                c.verified_date
            ));
        }

        // The client's own manifest, where it has one, is the authority on its
        // version — a `compat.json` claiming 0.2.0 for a `package.json` that
        // says 0.0.1 would ship a vsix whose compatibility statement describes a
        // different release.
        for (manifest, found) in manifest_versions(root, dir) {
            if found != c.client_version {
                errors.push(format!(
                    "{at}: `client_version` is {:?} but {manifest} says {found:?}",
                    c.client_version
                ));
            }
        }
    }

    if !errors.is_empty() {
        // Generation from data that failed validation would write a lie into
        // two clients and a documentation table.
        return errors;
    }

    // --- the three generated artifacts ----------------------------------
    let mut generated: Vec<(PathBuf, String)> = Vec::new();
    if let Some(c) = parsed.get("nvim") {
        generated.push((
            root.join("clients")
                .join("nvim")
                .join("lua")
                .join("wolf")
                .join("compat.lua"),
            nvim_lua(c),
        ));
    }
    if let Some(c) = parsed.get("vscode") {
        generated.push((
            root.join("clients")
                .join("vscode")
                .join("src")
                .join("compat.ts"),
            vscode_ts(c),
        ));
    }

    let doc = root.join("docs").join("COMPAT.md");
    match std::fs::read_to_string(&doc) {
        Ok(text) => match splice(&text, &table(&parsed)) {
            Ok(updated) => generated.push((doc, updated)),
            Err(e) => errors.push(format!("{}: {e}", slash(&doc))),
        },
        Err(e) => errors.push(format!("{}: {e}", slash(&doc))),
    }

    for (path, want) in generated {
        let have = std::fs::read_to_string(&path).unwrap_or_default();
        if have == want {
            continue;
        }
        if write {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&path, &want) {
                Ok(()) => eprintln!("compat: wrote {}", slash(&path)),
                Err(e) => errors.push(format!("{}: {e}", slash(&path))),
            }
        } else {
            errors.push(format!(
                "{} is stale — regenerate with `cargo xtask compat-generate` \
                 (it is GENERATED from clients/*/compat.json)",
                slash(&path)
            ));
        }
    }

    errors
}

/// Every wolf version this repository holds evidence for.
///
/// A `Vec` and not a scalar, and the shape is the load-bearing part: the day a
/// second wolf release exists and a second transcript set is recorded against
/// it, this function grows a second input and every caller is already written
/// against a set. Today the derivation has exactly one, because the repo pins
/// one commit and every transcript under `transcripts/` was recorded at it.
fn earned_versions(pin_version: &str) -> Result<Vec<String>, String> {
    let pin_number = version_number(pin_version).ok_or_else(|| {
        format!(
            "PIN: `version` is {pin_version:?}, which has no `MAJOR.MINOR.PATCH` in it \
             — the earned set is derived from it and cannot be built"
        )
    })?;
    Ok(Vec::from([pin_number]))
}

// ------------------------------------------------------------- parsing --

fn parse(dir: &str, text: &str) -> Result<Compat, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;

    let s = |path: &[&str]| -> Result<String, String> {
        let mut cur = &v;
        for key in path {
            cur = cur
                .get(*key)
                .ok_or_else(|| format!("missing `{}`", path.join(".")))?;
        }
        cur.as_str()
            .map(str::to_string)
            .ok_or_else(|| format!("`{}` must be a string", path.join(".")))
    };

    let evidence = v
        .get("verified")
        .and_then(|x| x.get("evidence"))
        .and_then(|x| x.as_array())
        .ok_or("missing `verified.evidence` (an array)")?
        .iter()
        .map(|x| x.as_str().unwrap_or_default().to_string())
        .collect();

    Ok(Compat {
        dir: dir.to_string(),
        client: s(&["client"])?,
        client_version: s(&["client_version"])?,
        artifact: s(&["artifact"])?,
        min: s(&["wolf", "min"])?,
        max_tested: s(&["wolf", "max_tested"])?,
        pin: s(&["wolf", "pin"])?,
        pin_version_string: s(&["wolf", "pin_version_string"])?,
        verified_date: s(&["verified", "date"])?,
        verified_host: s(&["verified", "host"])?,
        evidence,
        caveat: v
            .get("verified")
            .and_then(|x| x.get("caveat"))
            .and_then(|x| x.as_str())
            .map(str::to_string),
        warning_surface: s(&["warning_surface"])?,
    })
}

/// Version manifests a client keeps of its own, and the version each states.
fn manifest_versions(root: &Path, dir: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let client = root.join("clients").join(dir);

    if let Ok(text) = std::fs::read_to_string(client.join("package.json"))
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(found) = v.get("version").and_then(|x| x.as_str())
    {
        out.push((format!("clients/{dir}/package.json"), found.to_string()));
    }

    // `extension.toml` and `Cargo.toml` both carry `version = "…"`; read the
    // first such line rather than taking a TOML dependency (ls00 §7).
    for name in ["extension.toml", "Cargo.toml"] {
        if let Ok(text) = std::fs::read_to_string(client.join(name))
            && let Some(found) = first_version_key(&text)
        {
            out.push((format!("clients/{dir}/{name}"), found));
        }
    }
    out
}

fn first_version_key(text: &str) -> Option<String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "version").then(|| v.trim().trim_matches('"').to_string())
        })
}

/// `wolf 0.0.1 (pre-alpha)` → `0.0.1`.
pub fn version_number(version_string: &str) -> Option<String> {
    version_string
        .split_whitespace()
        .find(|w| triple(w).is_some())
        .map(str::to_string)
}

/// `0.0.1` → `(0, 0, 1)`, for ordering. `None` when it is not a triple.
pub fn triple(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.split('.');
    let a = parts.next()?.parse().ok()?;
    let b = parts.next()?.parse().ok()?;
    let c = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((a, b, c))
}

fn is_iso_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && s.chars().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        })
}

// ---------------------------------------------------------- generation --

fn nvim_lua(c: &Compat) -> String {
    format!(
        r#"-- GENERATED by `cargo xtask compat-generate` from clients/nvim/compat.json.
-- Do not edit; `cargo xtask compat-check` fails on a hand edit.
--
-- The range of `wolf` this plugin CLAIMS, as opposed to `pin.lua`, which is the
-- single build it was verified against. Pre-1.0 the two are the same width and
-- the docs say so (docs/COMPAT.md): until wolf v1 this is a PIN RANGE, and
-- breakage between compiler campaigns is expected rather than surprising.
--
-- `:checkhealth wolf` reads this. It reports a mismatch and never blocks: an
-- out-of-range server usually mostly works, and refusing to attach would be a
-- worse outcome than a warning nobody had to read twice.
return {{
  client = "{client}",
  client_version = "{client_version}",
  min = "{min}",
  max_tested = "{max_tested}",
  verified = "{verified_date}",
}}
"#,
        client = c.client,
        client_version = c.client_version,
        min = c.min,
        max_tested = c.max_tested,
        verified_date = c.verified_date,
    )
}

fn vscode_ts(c: &Compat) -> String {
    format!(
        r#"// GENERATED by `cargo xtask compat-generate` from clients/vscode/compat.json.
// Do not edit; `cargo xtask compat-check` fails on a hand edit.
//
// The range of `wolf` this extension CLAIMS, as opposed to `pin.ts`, which is
// the single build it was verified against. Pre-1.0 the two are the same width
// and the docs say so (docs/COMPAT.md): until wolf v1 this is a PIN RANGE, and
// breakage between compiler campaigns is expected rather than surprising.
//
// `activate()` reads this and shows AT MOST ONE notification per session on a
// mismatch. No modal, no repetition, no auto-update, and no refusal to start.

export const COMPAT = {{
	client: '{client}',
	clientVersion: '{client_version}',
	min: '{min}',
	maxTested: '{max_tested}',
	verified: '{verified_date}',
}} as const;
"#,
        client = c.client,
        client_version = c.client_version,
        min = c.min,
        max_tested = c.max_tested,
        verified_date = c.verified_date,
    )
}

const TABLE_BEGIN: &str =
    "<!-- compat-table-begin — GENERATED by `cargo xtask compat-generate` -->";
const TABLE_END: &str = "<!-- compat-table-end -->";

fn table(clients: &BTreeMap<String, Compat>) -> String {
    let mut out = String::new();
    out.push_str("| client | version | artifact | wolf min | wolf max_tested | pin | verified | warning surface |\n");
    out.push_str("|---|---|---|---|---|---|---|---|\n");
    for c in clients.values() {
        out.push_str(&format!(
            "| [{client}](../clients/{dir}/README.md) | {version} | {artifact} | {min} | {max} | `{pin}` | {date} ({host}) | {surface} |\n",
            client = c.client,
            dir = c.dir,
            version = c.client_version,
            artifact = c.artifact,
            min = c.min,
            max = c.max_tested,
            pin = &c.pin[..7],
            date = c.verified_date,
            host = c.verified_host,
            surface = c.warning_surface,
        ));
    }
    for c in clients.values() {
        if let Some(caveat) = &c.caveat {
            out.push_str(&format!("\n**`{}` caveat.** {caveat}\n", c.client));
        }
    }
    out
}

fn splice(doc: &str, block: &str) -> Result<String, String> {
    let start = doc
        .find(TABLE_BEGIN)
        .ok_or("no `compat-table-begin` marker — the generated table has nowhere to go")?;
    let end = doc.find(TABLE_END).ok_or("no `compat-table-end` marker")?;
    if end < start {
        return Err("the compat-table markers are in the wrong order".to_string());
    }
    Ok(format!(
        "{}{}\n\n{}{}",
        &doc[..start],
        TABLE_BEGIN,
        block,
        &doc[end..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_string_yields_its_number() {
        assert_eq!(
            version_number("wolf 0.0.1 (pre-alpha)").as_deref(),
            Some("0.0.1")
        );
        assert_eq!(version_number("wolf 1.2.30").as_deref(), Some("1.2.30"));
        assert_eq!(version_number("wolf pre-alpha"), None);
    }

    #[test]
    fn triples_order_numerically_not_lexically() {
        // The bug this exists to prevent: "0.10.0" < "0.9.0" as strings.
        assert!(triple("0.10.0") > triple("0.9.0"));
        assert_eq!(triple("0.0.1.2"), None);
        assert_eq!(triple("0.0"), None);
    }

    #[test]
    fn dates_are_iso_or_rejected() {
        assert!(is_iso_date("2026-08-10"));
        assert!(!is_iso_date("2026-8-10"));
        assert!(!is_iso_date("10/08/2026"));
    }

    #[test]
    fn splicing_is_idempotent_and_needs_both_markers() {
        let doc = format!("head\n\n{TABLE_BEGIN}\n\nold\n\n{TABLE_END}\ntail\n");
        let once = splice(&doc, "new\n").expect("markers present");
        let twice = splice(&once, "new\n").expect("markers survive");
        assert_eq!(once, twice);
        assert!(once.contains("new"));
        assert!(!once.contains("old"));
        assert!(splice("no markers", "x").is_err());
    }
}
