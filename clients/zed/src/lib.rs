//! The Zed extension for wolf — and the only reason it is code rather than
//! TOML.
//!
//! Zed's manifest can declare a language server exists, but it cannot say how
//! to *find* one: binary discovery is `language_server_command`, a function
//! compiled to WebAssembly and called by Zed at connect time. So this file is
//! the entire delta between "the config tier" and "a Rust crate", and it is
//! kept to the smallest thing that answers the question.
//!
//! What it deliberately does NOT do, and none of these is an oversight:
//!
//! - **No download.** Every other Zed language extension resolves a server by
//!   fetching a release from GitHub and caching it. `wolf lsp` *is* the compiler
//!   (D34), so there is nothing separate to install and nothing to keep in sync
//!   with a toolchain version. `zed_extension_api`'s `download_file`,
//!   `latest_github_release` and `set_language_server_installation_status` are
//!   all unused, and the `cached_binary_path` field that every other extension
//!   carries does not exist here because there is no download to cache.
//! - **No `language_server_initialization_options`.** `wolf lsp` reads no
//!   settings, so sending an empty object would only add a thing to keep true.
//! - **No `language_server_workspace_configuration`.** Same reason, and a
//!   stronger one: `wolf lsp` sends no server→client requests at all, so it
//!   never asks for configuration (docs/SERVER-CONSTRAINTS.md, facsimile).
//! - **No capability trimming, no middleware, no diagnostic post-processing.**
//!   D22 makes the compiler's diagnostics the reviewed artifact; a client that
//!   rewrote one would become a second, unreviewed authority on what the
//!   compiler said. The way to not be that is to have nowhere to put the code.

use zed_extension_api::{self as zed, LanguageServerId, Result, settings::LspSettings};

struct WolfExtension;

/// The subcommand that turns the compiler into a language server (D34).
const LSP_ARGS: &[&str] = &["lsp"];

/// The manifest's `[language_servers.wolf]` key, which is also the id a user
/// writes under `"lsp"` in `settings.json`. Zed passes it back to us as
/// `language_server_id`; we compare rather than assume, so a rename of the
/// manifest key fails loudly here instead of silently reading nobody's settings.
const SERVER_ID: &str = "wolf";

impl zed::Extension for WolfExtension {
    fn new() -> Self {
        // No state. Every other extension caches a downloaded binary path here;
        // this one has nothing to cache, and a struct with no fields is the
        // honest encoding of that.
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let id = language_server_id.as_ref();
        if id != SERVER_ID {
            return Err(format!(
                "wolf extension asked to start unknown language server `{id}` \
                 (this extension registers `{SERVER_ID}` and nothing else)"
            ));
        }

        // The environment is the worktree's shell environment, unmodified. It is
        // passed through rather than curated because `wolf` resolves its
        // toolchain from the ambient environment exactly as it does on a command
        // line, and an extension that filtered it would make `wolf lsp` behave
        // differently under Zed than in a terminal.
        let env = worktree.shell_env();

        // 1. An explicit override in `settings.json` wins, always:
        //
        //      { "lsp": { "wolf": { "binary": { "path": "/abs/path/to/wolf" } } } }
        //
        //    Zed requires that path be absolute. `arguments` may override the
        //    subcommand too — someone wrapping `wolf` in a launcher needs that —
        //    but the default when the key is absent is `["lsp"]` rather than
        //    empty, because a `wolf` invoked with no subcommand is not a server.
        if let Ok(settings) = LspSettings::for_worktree(SERVER_ID, worktree) {
            if let Some(binary) = settings.binary {
                if let Some(path) = binary.path {
                    return Ok(zed::Command {
                        command: path,
                        args: binary
                            .arguments
                            .unwrap_or_else(|| LSP_ARGS.iter().map(|s| s.to_string()).collect()),
                        env,
                    });
                }
            }
        }

        // 2. Otherwise `wolf` on `PATH`, resolved through the worktree so that a
        //    per-project shell environment (direnv, a toolchain shim) is
        //    honoured — `which` here is Zed's, not the host's bare `PATH`.
        //
        //    There is no third option and no auto-download. The error text names
        //    both remedies, because "server failed to start" with no explanation
        //    is the failure mode this whole function exists to avoid.
        let command = worktree.which("wolf").ok_or_else(|| {
            "`wolf` was not found in this worktree's PATH. Install the wolf \
             toolchain and put `wolf` on PATH, or set an absolute path in \
             settings.json: {\"lsp\": {\"wolf\": {\"binary\": {\"path\": \"…\"}}}}"
                .to_string()
        })?;

        Ok(zed::Command {
            command,
            args: LSP_ARGS.iter().map(|s| s.to_string()).collect(),
            env,
        })
    }
}

zed::register_extension!(WolfExtension);
