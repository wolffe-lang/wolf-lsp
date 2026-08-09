//! `vendor/upstream/PIN` — the wolf-lang commit this repo is keyed to.
//!
//! wolf-interp's PIN is a bare sha. This one carries two more fields, because
//! ls00 §3 asks it to answer a question a sha cannot: *is the binary I found
//! actually the one at the pin, and does it serve LSP at all?*
//!
//! - `version` is the exact `wolf --version` string the pinned commit
//!   produces. `lspconf doctor` fails on a mismatch — a stale local `wolf`
//!   producing green transcripts is the precise failure mode this exists to
//!   prevent.
//! - `serves_lsp` records whether `wolf lsp` exists at this pin. It does not
//!   yet: the subcommand lands with wolf-lang's s52, which is queued behind
//!   s17. Encoding that in **data** rather than in a `cfg` or a comment means
//!   the day it flips is a one-line pin-bump diff, and until then every
//!   server-dependent path skips for a reason it can print.

use std::fmt;
use std::path::Path;

/// A parsed PIN file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    /// Full 40-character wolf-lang commit sha.
    pub commit: String,
    /// The `wolf --version` string that commit produces.
    pub version: String,
    /// Whether `wolf lsp` exists at this commit.
    pub serves_lsp: bool,
}

/// Why a PIN file could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Missing(String),
    Unreadable(String),
    Malformed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Missing(p) => write!(
                f,
                "no PIN at {p} — this repo does not know which wolf-lang commit it targets"
            ),
            Error::Unreadable(m) => write!(f, "PIN unreadable: {m}"),
            Error::Malformed(m) => write!(f, "PIN malformed: {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl Pin {
    /// Read `<repo_root>/vendor/upstream/PIN`.
    pub fn load(repo_root: &Path) -> Result<Self, Error> {
        let path = repo_root.join("vendor").join("upstream").join("PIN");
        if !path.is_file() {
            return Err(Error::Missing(path.display().to_string()));
        }
        let text = std::fs::read_to_string(&path).map_err(|e| Error::Unreadable(e.to_string()))?;
        Self::parse(&text)
    }

    /// Parse the `key = value` body. Comments start with `#`; blank lines are
    /// ignored; unknown keys are an error, because a typo'd key that silently
    /// did nothing is exactly how a pin stops meaning anything.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let (mut commit, mut version, mut serves_lsp) = (None, None, None);
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(Error::Malformed(format!(
                    "line {}: not `key = value`: {raw:?}",
                    n + 1
                )));
            };
            let value = value.trim().trim_matches('"').to_string();
            match key.trim() {
                "commit" => commit = Some(value),
                "version" => version = Some(value),
                "serves_lsp" => {
                    serves_lsp = Some(match value.as_str() {
                        "true" => true,
                        "false" => false,
                        other => {
                            return Err(Error::Malformed(format!(
                                "line {}: serves_lsp must be true or false, got {other:?}",
                                n + 1
                            )));
                        }
                    });
                }
                other => {
                    return Err(Error::Malformed(format!(
                        "line {}: unknown key {other:?}",
                        n + 1
                    )));
                }
            }
        }

        let commit = commit.ok_or_else(|| Error::Malformed("no `commit`".to_string()))?;
        if commit.len() != 40 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::Malformed(format!(
                "`commit` must be a full 40-char sha, got {commit:?} \
                 — an abbreviated sha is a moving target as upstream grows"
            )));
        }
        let version = version.ok_or_else(|| Error::Malformed("no `version`".to_string()))?;
        if version.is_empty() {
            return Err(Error::Malformed("`version` is empty".to_string()));
        }
        let serves_lsp =
            serves_lsp.ok_or_else(|| Error::Malformed("no `serves_lsp`".to_string()))?;

        Ok(Self {
            commit,
            version,
            serves_lsp,
        })
    }

    /// Short sha, for messages.
    #[must_use]
    pub fn short(&self) -> &str {
        &self.commit[..7]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "\
# a comment
commit = ecea37c312595bc7e8fbd20d1240200e1091e234
version = \"wolf 0.0.1 (pre-alpha)\"
serves_lsp = false
";

    #[test]
    fn parses_the_committed_shape() {
        let pin = Pin::parse(GOOD).unwrap();
        assert_eq!(pin.commit, "ecea37c312595bc7e8fbd20d1240200e1091e234");
        assert_eq!(pin.version, "wolf 0.0.1 (pre-alpha)");
        assert!(!pin.serves_lsp);
        assert_eq!(pin.short(), "ecea37c");
    }

    #[test]
    fn an_abbreviated_sha_is_refused() {
        let err = Pin::parse("commit = ecea37c\nversion = v\nserves_lsp = false\n").unwrap_err();
        assert!(
            matches!(err, Error::Malformed(ref m) if m.contains("40-char")),
            "{err}"
        );
    }

    #[test]
    fn a_typod_key_is_an_error_not_a_shrug() {
        let err = Pin::parse("commit_sha = x\n").unwrap_err();
        assert!(
            matches!(err, Error::Malformed(ref m) if m.contains("unknown key")),
            "{err}"
        );
    }

    #[test]
    fn every_field_is_required() {
        for missing in ["commit", "version", "serves_lsp"] {
            let text: String = GOOD
                .lines()
                .filter(|l| !l.trim_start().starts_with(missing))
                .collect::<Vec<_>>()
                .join("\n");
            let err = Pin::parse(&text).unwrap_err();
            assert!(
                matches!(&err, Error::Malformed(m) if m.contains(missing)),
                "dropping {missing} gave {err}"
            );
        }
    }
}
