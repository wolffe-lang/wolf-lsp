//! `lspconf doctor` — is there a server, is it the right one, and if not, say
//! so in a sentence someone can act on.
//!
//! This is the module that keeps a dark CI lane from being a lane nobody
//! notices is dark (ls00 §3). Every server-dependent step in this repo asks
//! [`Doctor::availability`] first, and when the answer is not [`Availability::Ready`]
//! it prints the `SKIP:` line *with the reason* and exits `77`. Silence is the
//! failure mode; a skip that names its cause is not.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::locate::{self, Located};
use crate::pin::{self, Pin};

/// The state of the world with respect to a runnable `wolf lsp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// No usable `vendor/upstream/PIN`. Nothing can be checked against
    /// anything; this is a repo error, not a skip.
    NoPin(pin::Error),
    /// Resolution found nothing. The expected state in CI until wolf-lang
    /// publishes a `xtask dist` artifact.
    NoBinary,
    /// A binary is there but would not report a version.
    Unversioned { path: PathBuf, reason: String },
    /// A binary is there and is the wrong one. **Not** a skip: a stale local
    /// `wolf` producing green transcripts is precisely the outcome the pin
    /// exists to prevent, so this is a failure.
    VersionMismatch {
        path: PathBuf,
        found: String,
        expected: String,
    },
    /// The binary matches the pin, but `wolf lsp` does not exist at that pin.
    /// Today's state: the subcommand lands with wolf-lang's s52, queued behind
    /// s17. A skip, and the honest one.
    PinPredatesLsp { path: PathBuf, version: String },
    /// A server at the pin, ready to be driven.
    Ready { path: PathBuf },
}

impl Availability {
    /// Can a server-dependent step run?
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self, Availability::Ready { .. })
    }

    /// Is this a state a step should *skip* over (exit 77), as opposed to
    /// fail on?
    ///
    /// The split is the whole design: absence is a skip, *wrongness* is a
    /// failure. A missing binary means the artifact does not exist yet; a
    /// mismatched one means someone is about to test the wrong software.
    #[must_use]
    pub fn is_skip(&self) -> bool {
        matches!(
            self,
            Availability::NoBinary | Availability::PinPredatesLsp { .. }
        )
    }

    /// The one-line reason, for the `SKIP:` or `FATAL:` line.
    #[must_use]
    pub fn reason(&self, pin: Option<&Pin>) -> String {
        let at = pin.map_or_else(|| "<unknown pin>".to_string(), |p| p.short().to_string());
        match self {
            Availability::NoPin(e) => format!("{e}"),
            Availability::NoBinary => format!(
                "no wolf binary at pin {at} \
                 (checked $WOLF_BIN, .wolf-bin/, PATH; wolf-lang publishes no release artifact yet)"
            ),
            Availability::Unversioned { path, reason } => {
                format!("{} would not report a version: {reason}", path.display())
            }
            Availability::VersionMismatch {
                path,
                found,
                expected,
            } => format!(
                "{} reports {found:?} but pin {at} is {expected:?} \
                 — testing the wrong binary is worse than testing none",
                path.display()
            ),
            Availability::PinPredatesLsp { path, .. } => format!(
                "no `wolf lsp` at pin {at}: {} is the pinned binary, but the subcommand \
                 lands with wolf-lang's s52 (queued behind s17)",
                path.display()
            ),
            Availability::Ready { path } => format!("{} serves LSP at pin {at}", path.display()),
        }
    }
}

/// A full diagnosis.
#[derive(Debug, Clone)]
pub struct Doctor {
    pub pin: Option<Pin>,
    pub located: Option<Located>,
    pub availability: Availability,
}

impl Doctor {
    /// Diagnose, spawning the located binary once with `--version`.
    #[must_use]
    pub fn run(repo_root: &Path) -> Self {
        let pin = match Pin::load(repo_root) {
            Ok(p) => p,
            Err(e) => {
                return Self {
                    pin: None,
                    located: None,
                    availability: Availability::NoPin(e),
                };
            }
        };

        let Some(located) = locate::locate_server(repo_root) else {
            return Self {
                pin: Some(pin),
                located: None,
                availability: Availability::NoBinary,
            };
        };

        let availability = match query_version(&located.path) {
            Err(reason) => Availability::Unversioned {
                path: located.path.clone(),
                reason,
            },
            Ok(found) if found != pin.version => Availability::VersionMismatch {
                path: located.path.clone(),
                found,
                expected: pin.version.clone(),
            },
            Ok(version) if !pin.serves_lsp => Availability::PinPredatesLsp {
                path: located.path.clone(),
                version,
            },
            Ok(_) => Availability::Ready {
                path: located.path.clone(),
            },
        };

        Self {
            pin: Some(pin),
            located: Some(located),
            availability,
        }
    }

    /// The `SKIP:` / `FATAL:` line a server-dependent step should print, or
    /// `None` when the server is ready.
    #[must_use]
    pub fn skip_line(&self) -> Option<String> {
        if self.availability.is_ready() {
            return None;
        }
        let tag = if self.availability.is_skip() {
            "SKIP"
        } else {
            "FATAL"
        };
        Some(format!(
            "{tag}: {}",
            self.availability.reason(self.pin.as_ref())
        ))
    }
}

impl fmt::Display for Doctor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "lspconf doctor")?;
        match &self.pin {
            Some(p) => {
                writeln!(f, "  pin        {} ({})", p.commit, p.short())?;
                writeln!(f, "  expects    {:?}", p.version)?;
                writeln!(
                    f,
                    "  wolf lsp   {}",
                    if p.serves_lsp {
                        "exists at this pin"
                    } else {
                        "ABSENT at this pin (wolf-lang s52)"
                    }
                )?;
            }
            None => writeln!(f, "  pin        <unreadable>")?,
        }
        match &self.located {
            Some(Located { path, source }) => {
                writeln!(f, "  binary     {} (via {source})", path.display())?;
            }
            None => writeln!(
                f,
                "  binary     none ($WOLF_BIN, .wolf-bin/, PATH all empty)"
            )?,
        }
        write!(f, "  verdict    ")?;
        if self.availability.is_ready() {
            writeln!(f, "READY — {}", self.availability.reason(self.pin.as_ref()))
        } else if self.availability.is_skip() {
            writeln!(
                f,
                "SERVER UNAVAILABLE — {}",
                self.availability.reason(self.pin.as_ref())
            )
        } else {
            writeln!(f, "ERROR — {}", self.availability.reason(self.pin.as_ref()))
        }
    }
}

/// Run `<bin> --version` and return its first stdout line, trimmed.
fn query_version(path: &Path) -> Result<String, String> {
    let out = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("exited {}", out.status));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let first = text.lines().next().unwrap_or("").trim();
    if first.is_empty() {
        return Err("printed nothing on stdout".to_string());
    }
    Ok(first.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(serves_lsp: bool) -> Pin {
        Pin {
            commit: "ecea37c312595bc7e8fbd20d1240200e1091e234".to_string(),
            version: "wolf 0.0.1 (pre-alpha)".to_string(),
            serves_lsp,
        }
    }

    #[test]
    fn absence_skips_and_wrongness_fails() {
        assert!(Availability::NoBinary.is_skip());
        assert!(
            Availability::PinPredatesLsp {
                path: PathBuf::from("wolf"),
                version: "wolf 0.0.1 (pre-alpha)".to_string(),
            }
            .is_skip()
        );
        // A binary at the wrong version must never be skipped past.
        assert!(
            !Availability::VersionMismatch {
                path: PathBuf::from("wolf"),
                found: "wolf 9.9.9".to_string(),
                expected: "wolf 0.0.1 (pre-alpha)".to_string(),
            }
            .is_skip()
        );
    }

    #[test]
    fn every_non_ready_state_names_its_cause() {
        let p = pin(false);
        for a in [
            Availability::NoBinary,
            Availability::Unversioned {
                path: PathBuf::from("/x/wolf"),
                reason: "exited 2".to_string(),
            },
            Availability::VersionMismatch {
                path: PathBuf::from("/x/wolf"),
                found: "wolf 9.9.9".to_string(),
                expected: p.version.clone(),
            },
            Availability::PinPredatesLsp {
                path: PathBuf::from("/x/wolf"),
                version: p.version.clone(),
            },
        ] {
            let reason = a.reason(Some(&p));
            assert!(!reason.is_empty(), "{a:?} produced an empty reason");
            assert!(!a.is_ready());
        }
    }

    #[test]
    fn a_repo_with_no_pin_reports_that_rather_than_guessing() {
        let tmp = std::env::temp_dir().join("wolf-lsp-doctor-no-pin");
        std::fs::create_dir_all(&tmp).unwrap();
        let doc = Doctor::run(&tmp);
        assert!(
            matches!(doc.availability, Availability::NoPin(_)),
            "{:?}",
            doc.availability
        );
        assert!(doc.skip_line().unwrap().starts_with("FATAL:"));
    }
}
