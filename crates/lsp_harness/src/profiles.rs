//! Capability profiles — what a client *declares*, and what the server may
//! therefore promise.
//!
//! Sprint §4 puts a hard line under this file: a profile is **derived from a
//! real client**, and records the commit it was read from. A profile invented
//! here rather than read off a client is a fiction the suite then tests
//! against, which is worse than having no profile at all — it produces a green
//! lane for a client nobody checked.
//!
//! So profiles come in exactly two flavors, and the file says which it is:
//!
//! - **`synthetic`** — a capability document that claims to be nobody. The
//!   `minimal`/`maximal` pair the sprint names, plus the encoding-negotiation
//!   documents §5 needs. These are honest because they make no claim about an
//!   editor; `minimal` is not "some client we didn't check", it is *the*
//!   client that declares nothing.
//! - **`derived`** — read off a named client at a named commit on a named
//!   date. ls02–ls06 produce these. A `derived` profile missing any of those
//!   three fields fails validation, because that is the exact shape a fiction
//!   would take.
//!
//! [`missing_derived`] is the other half of the honesty: it reports which of
//! the sprint's six real clients still have no profile and which sprint owes
//! each one, so "eight profiles validated" cannot quietly mean "two profiles
//! validated and six forgotten".

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use lsp_transcript::Encoding;

/// Where a profile's capabilities came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Provenance {
    /// Constructed to exercise a protocol case; claims to be no editor.
    Synthetic {
        /// What the document is *for*. Required — a synthetic profile with no
        /// stated purpose is an arbitrary one.
        purpose: String,
    },
    /// Read off a real client, at a commit, on a date.
    Derived {
        client: String,
        repository: String,
        commit: String,
        read_on: String,
        /// The file(s) in the client the declaration was read from.
        source: Vec<String>,
    },
}

/// One client-capability document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Must equal the file stem, so a rename cannot silently retarget a
    /// transcript's `profile` field.
    pub profile: String,
    pub provenance: Provenance,
    /// The encoding this profile is expected to negotiate, as an independent
    /// statement of intent.
    ///
    /// Not read off the server — that would make the assertion "the server
    /// agrees with itself". Writing the expectation here means a change in the
    /// server's negotiation *rule* fails the suite even when it stays
    /// self-consistent, which is the only way that rule is testable at all.
    pub expects_encoding: Encoding,
    /// The LSP `ClientCapabilities` object, verbatim.
    pub capabilities: Value,
}

/// The real clients this repo tracks, and the sprint that owes each a profile.
///
/// ls01 §4 named six. `emacs` is a seventh, added by ls06 and recorded as a
/// delta: eglot turned out to be drivable under `emacs --batch`, so there is a
/// real session to read a profile off — and a tracked client whose profile
/// nothing watches is exactly the gap this list exists to close. Membership
/// here is a claim about *tracking*, not about tier: `emacs` is T2 in
/// `docs/MATRIX.md`, and `zed` is T2 with its profile still owed.
pub const REAL_CLIENTS: &[(&str, &str)] = &[
    ("fackr", "ls02"),
    ("facsimile", "ls03"),
    ("nvim", "ls04"),
    ("vscode", "ls05"),
    ("helix", "ls06"),
    ("zed", "ls06"),
    ("emacs", "ls06"),
];

/// A profile that did not load or did not hold up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub file: String,
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

impl std::error::Error for Error {}

impl Profile {
    /// Read and validate one profile document.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let file = crate::slash_path(path);
        let err = |m: String| Error {
            file: file.clone(),
            message: m,
        };
        let text = std::fs::read_to_string(path).map_err(|e| err(e.to_string()))?;
        let profile: Profile = serde_json::from_str(&text).map_err(|e| err(e.to_string()))?;
        profile.validate(path)?;
        Ok(profile)
    }

    /// Structural checks that need no server.
    pub fn validate(&self, path: &Path) -> Result<(), Error> {
        let file = crate::slash_path(path);
        let err = |m: String| Error {
            file: file.clone(),
            message: m,
        };
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if stem != self.profile {
            return Err(err(format!(
                "the document calls itself `{}` but the file is `{stem}.json` \
                 — a transcript naming one would silently get the other",
                self.profile
            )));
        }
        if !self.capabilities.is_object() {
            return Err(err("`capabilities` must be a JSON object".to_string()));
        }
        match &self.provenance {
            Provenance::Synthetic { purpose } if purpose.trim().is_empty() => {
                return Err(err(
                    "a synthetic profile must state its purpose — one with no stated \
                     purpose is an arbitrary one"
                        .to_string(),
                ));
            }
            Provenance::Derived {
                client,
                repository,
                commit,
                read_on,
                source,
            } => {
                for (name, value) in [
                    ("client", client.as_str()),
                    ("repository", repository.as_str()),
                    ("commit", commit.as_str()),
                    ("read_on", read_on.as_str()),
                ] {
                    if value.trim().is_empty() {
                        return Err(err(format!(
                            "a derived profile must record `{name}` \
                             — a profile that drifts from its client is a lie the suite tells"
                        )));
                    }
                }
                if source.is_empty() {
                    return Err(err(
                        "a derived profile must name the file(s) it was read from".to_string(),
                    ));
                }
            }
            Provenance::Synthetic { .. } => {}
        }
        // The declared expectation must be consistent with the declaration
        // itself: a profile that offers only utf-32 cannot expect utf-8.
        let offered = self.offered_encodings();
        if !offered.is_empty() && !offered.contains(&self.expects_encoding) {
            return Err(err(format!(
                "`expects_encoding` is {} but the profile offers only [{}] \
                 — the protocol forbids a server answering with an encoding \
                 the client did not offer, except the utf-16 default",
                self.expects_encoding,
                offered
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        if offered.is_empty() && self.expects_encoding != Encoding::Utf16 {
            return Err(err(format!(
                "a profile that offers no `positionEncodings` must expect utf-16 \
                 (the protocol's mandatory default), not {}",
                self.expects_encoding
            )));
        }
        Ok(())
    }

    /// The encodings this profile declares, in the order it declares them.
    ///
    /// Unknown names are skipped rather than refused: a client offering
    /// `utf-7` is a real case the negotiation has to survive, and it is
    /// exercised by a profile whose whole point is offering nothing usable.
    #[must_use]
    pub fn offered_encodings(&self) -> Vec<Encoding> {
        self.capabilities
            .pointer("/general/positionEncodings")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .filter_map(|s| s.parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Did the client declare *anything* under `positionEncodings`, known or
    /// not? Distinguishes "offered nothing" from "offered only nonsense",
    /// which negotiate differently in principle and identically in this
    /// server — a fact the suite pins rather than assumes.
    #[must_use]
    pub fn declares_any_encoding(&self) -> bool {
        self.capabilities
            .pointer("/general/positionEncodings")
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
    }

    /// Whether the profile declared dynamic registration anywhere.
    ///
    /// The classic trap §4 names: a server that advertises a capability whose
    /// *delivery* needs `client/registerCapability` against a client that
    /// never declared it advertises something it cannot deliver.
    #[must_use]
    pub fn declares_dynamic_registration(&self) -> bool {
        fn walk(v: &Value) -> bool {
            match v {
                Value::Object(m) => m.iter().any(|(k, v)| {
                    (k == "dynamicRegistration" && v.as_bool() == Some(true)) || walk(v)
                }),
                Value::Array(a) => a.iter().any(walk),
                _ => false,
            }
        }
        walk(&self.capabilities)
    }
}

/// Load every `profiles/*.json`, keyed by name.
pub fn load_all(repo_root: &Path) -> (BTreeMap<String, Profile>, Vec<Error>) {
    let dir = repo_root.join("profiles");
    let mut out = BTreeMap::new();
    let mut errors = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return (out, errors);
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        match Profile::load(&path) {
            Ok(p) => {
                out.insert(p.profile.clone(), p);
            }
            Err(e) => errors.push(e),
        }
    }
    (out, errors)
}

/// The real-client profiles that do not exist yet, with the sprint that owes
/// each — so "all profiles validated" can never quietly mean "the two we have".
#[must_use]
pub fn missing_derived(loaded: &BTreeMap<String, Profile>) -> Vec<(&'static str, &'static str)> {
    REAL_CLIENTS
        .iter()
        .filter(|(name, _)| {
            !matches!(
                loaded.get(*name).map(|p| &p.provenance),
                Some(Provenance::Derived { .. })
            )
        })
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn synthetic(name: &str, encodings: Option<Vec<&str>>, expects: Encoding) -> Profile {
        let capabilities = match encodings {
            Some(list) => json!({"general": {"positionEncodings": list}}),
            None => json!({}),
        };
        Profile {
            profile: name.to_string(),
            provenance: Provenance::Synthetic {
                purpose: "a test".to_string(),
            },
            expects_encoding: expects,
            capabilities,
        }
    }

    #[test]
    fn a_profile_whose_name_disagrees_with_its_file_is_refused() {
        let p = synthetic("minimal", None, Encoding::Utf16);
        let err = p.validate(Path::new("profiles/maximal.json")).unwrap_err();
        assert!(err.message.contains("maximal"), "{err}");
    }

    #[test]
    fn a_profile_cannot_expect_an_encoding_it_never_offered() {
        let p = synthetic("x", Some(vec!["utf-32"]), Encoding::Utf8);
        let err = p.validate(Path::new("profiles/x.json")).unwrap_err();
        assert!(err.message.contains("did not offer"), "{err}");
    }

    #[test]
    fn offering_nothing_means_expecting_the_utf16_default() {
        assert!(
            synthetic("x", None, Encoding::Utf16)
                .validate(Path::new("profiles/x.json"))
                .is_ok()
        );
        let err = synthetic("x", None, Encoding::Utf8)
            .validate(Path::new("profiles/x.json"))
            .unwrap_err();
        assert!(err.message.contains("mandatory default"), "{err}");
    }

    #[test]
    fn a_derived_profile_missing_its_commit_is_refused() {
        // Exactly the shape a fiction would take: a real client's name with
        // no evidence anyone read it.
        let p = Profile {
            profile: "fackr".to_string(),
            provenance: Provenance::Derived {
                client: "fackr".to_string(),
                repository: "https://example.invalid/fackr".to_string(),
                commit: String::new(),
                read_on: "2026-08-09".to_string(),
                source: vec!["src/lsp/types.rs".to_string()],
            },
            expects_encoding: Encoding::Utf16,
            capabilities: json!({}),
        };
        let err = p.validate(Path::new("profiles/fackr.json")).unwrap_err();
        assert!(err.message.contains("commit"), "{err}");
    }

    #[test]
    fn dynamic_registration_is_found_at_any_depth() {
        let mut p = synthetic("x", None, Encoding::Utf16);
        assert!(!p.declares_dynamic_registration());
        p.capabilities = json!({"textDocument": {"formatting": {"dynamicRegistration": true}}});
        assert!(p.declares_dynamic_registration());
        p.capabilities = json!({"textDocument": {"formatting": {"dynamicRegistration": false}}});
        assert!(!p.declares_dynamic_registration());
    }

    #[test]
    fn unknown_encoding_names_are_skipped_not_refused() {
        let p = synthetic("x", Some(vec!["utf-7", "utf-16"]), Encoding::Utf16);
        assert_eq!(p.offered_encodings(), vec![Encoding::Utf16]);
        assert!(p.declares_any_encoding());
        assert!(p.validate(Path::new("profiles/x.json")).is_ok());
    }

    #[test]
    fn missing_derived_names_the_sprint_that_owes_each_client() {
        let mut loaded = BTreeMap::new();
        loaded.insert(
            "minimal".to_string(),
            synthetic("minimal", None, Encoding::Utf16),
        );
        let missing = missing_derived(&loaded);
        assert_eq!(missing.len(), REAL_CLIENTS.len());
        assert!(missing.contains(&("fackr", "ls02")));
    }

    #[test]
    fn a_synthetic_profile_standing_in_for_a_client_is_still_reported_missing() {
        // The failure this guards: someone writes `fackr.json` by guessing,
        // marks it synthetic to pass validation, and the suite then reports
        // fackr as covered.
        let mut loaded = BTreeMap::new();
        loaded.insert(
            "fackr".to_string(),
            synthetic("fackr", None, Encoding::Utf16),
        );
        assert!(missing_derived(&loaded).contains(&("fackr", "ls02")));
    }
}
