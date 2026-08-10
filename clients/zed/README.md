# zed

**Tier 2 — the config tier, and the row where that label is least true.**

Zed can register a *language* from TOML. It cannot register a *language server*
from TOML: binary discovery is `language_server_command`, a Rust function
compiled to a WebAssembly component and called by Zed at connect time. So this
directory is a crate, and the honest description of the tier is "a thin
extension whose only job is to find `wolf` on `PATH`".

- Upstream: `zed-industries/zed`, API read at `zed_extension_api 0.7.0`
  (newest version published to crates.io)
- Capability profile: **owed** — see "What has not been verified" below
- Recorded session: **owed** — same reason

## What is in here

```
extension.toml                    manifest: one language server, no grammars
Cargo.toml                        cdylib, zed_extension_api 0.7.0
src/lib.rs                        ~40 lines of glue and ~40 of reasons
languages/wolf/config.toml        .lu — comments, brackets, indent
languages/wolfi/config.toml       .wolfi — the same, with no server attached
```

There is no `highlights.scm` and no `[grammars.wolf]` table. Both wait on
`tree-sitter-wolf`, which is a seed commit with no `grammar.js`
(`b1b2c17`). For Zed this is not merely cosmetic: **Zed builds every grammar
named in `extension.toml` when the extension is installed**, so a block pointing
at an empty repository fails the install outright and takes the language server
down with it.

**Zed's `grammar` key is optional, and that is what makes this work.**
`LanguageConfig::grammar` is `Option<Arc<str>>` and defaults to `None`; Zed's
own built-in "Plain Text" language ships with no grammar at all. The registry
branches on it and attaches no queries when it is absent — no error, no warning.
So `languages/wolf/config.toml` deliberately has no `grammar` key, and comments,
brackets, autoclose and indent all still work. What is absent is syntax
highlighting, and only that.

## Setup

`wolf lsp` **is** the compiler (D34), so there is no server to install and no
version to keep in sync with anything.

1. Put `wolf` on `PATH`.
2. Install the extension (below).
3. Open a `.lu` file.

If `wolf` is not on `PATH`, point Zed at it in `settings.json` — the key is the
`[language_servers.wolf]` id from `extension.toml`, and Zed requires an
**absolute** path:

```json
{
  "lsp": {
    "wolf": {
      "binary": {
        "path": "/absolute/path/to/wolf",
        "arguments": ["lsp"]
      }
    }
  }
}
```

There is no third option and no auto-download.

### Installing, which today means a dev extension

There is **no Zed extension-registry listing** — ls07 owns publishing, exactly
as it owns the VS Code marketplace. Until then the install path is Zed's dev
extension flow:

1. `rustup target add wasm32-wasip2`
2. In Zed, open the command palette and run **`zed: install dev extension`**
   (the `zed::InstallDevExtension` action), then choose `clients/zed`.

Zed compiles the crate itself as part of that action; there is no `cargo build`
step for you to run first.

**That flow is GUI-only, and it is the reason two slots below are owed.** Zed's
CLI has no `--install-extension` and no `--dev-extension` flag — the full arg
list is `--wait --add --new --reuse --existing --classic --user-data-dir
--version --foreground --zed --dev-server-token --wsl --system-specs
--dev-container --diff --completions --uninstall --askpass` — and
`auto_install_extensions` in `settings.json` installs *published* extensions by
id, not dev extensions. There is no headless path in, so there is no headless
way out.

## What has NOT been verified, stated first

This is the least-verified row in `docs/MATRIX.md` and the README says so before
it says anything else.

**No `profiles/zed.json`, and none will be invented.** A capability profile is
read off a real session or it does not exist (`profiles/README.md`): "a profile
invented here rather than read off a client is a lie the suite then tests
against, which is worse than having no profile". Zed cannot be installed with a
dev extension without a GUI, cannot be scripted, and exposes no LSP trace file
to read a client-capabilities object out of. So `lspconf profiles` names `zed`
as owed on every run, and it will keep saying so until someone runs Zed on a
desktop with the capture shim on `PATH` and commits what came back.

**No `transcripts/zed/smoke.jsonl`**, for the same reason and with the same
remedy.

**The wasm component has never been built on this machine.** `cargo check`
against the **host** target passes — so the API usage against
`zed_extension_api 0.7.0` is type-correct, `Extension` is implemented with the
right signature, and `register_extension!` expands — but the real artifact is a
`wasm32-wasip2` component, and that target's `std` is not installed here (Arch
Linux ships `rust` without `rustup`, and adding the target would mean replacing
the system toolchain). CI does the wasm build, with `rustup target add
wasm32-wasip2`; the asymmetry is recorded in the matrix rather than hidden.

**The sprint says `wasm32-wasip1`; the correct target is `wasm32-wasip2`.** Zed's
`extension_builder.rs` has `const RUST_TARGET: &str = "wasm32-wasip2"` with the
comment "Currently, we compile with Rust's `wasm32-wasip2` target, which works
with WASI `preview2` and the component model", and the extension docs say the
same. This is a delta from ls06 §2 and belongs in the campaign closeout.

**The indent regexes have never been run.** `increase_indent_pattern` and
`decrease_indent_pattern` are compiled by Zed with the Rust `regex` crate, which
has no lookahead — so the `^((?!//).)*…` form in
`clients/vscode/language-configuration.json` could not be transliterated and a
lookahead-free approximation is shipped instead. The visible cost is that a `{`
inside a trailing line comment still increases indent. Nothing here exercises
them; only a human running Zed can.

## Known limitations — stated honestly

**A `.lu` buffer in Zed has no syntax highlighting today**, for the grammar
reason above. Everything a server provides still arrives.

**`.wolfi` is a language with no server attached.** `extension.toml` lists
`languages = ["Wolf"]` and not `"Wolfi"`, deliberately: `wolfi` v0 is a *binary*
format (magic bytes `WOLFI`,
`upstream/crates/wolf_sema/src/interface.rs`) and `wolf lsp` discovers modules by
`.lu` alone (D32). Same ruling as ls04, ls05 and `clients/helix`.

**No `language_server_initialization_options`, no
`language_server_workspace_configuration`, no capability trimming.** `wolf lsp`
reads no settings and sends no server→client requests at all
(`docs/SERVER-CONSTRAINTS.md`), so an empty object either way would only be a
thing to keep true. D22 forbids the client rewriting a diagnostic, and the way
to not do that is to have nowhere to put the code.

**No download, and no `cached_binary_path`.** Every other Zed language extension
resolves its server by fetching a GitHub release and caching the path. `wolf lsp`
is the compiler, so there is nothing separate to fetch — which is why
`WolfExtension` is a struct with no fields.

**`edition = "2021"`, not the workspace's 2024.** This crate is deliberately not
a workspace member (the root `Cargo.toml` excludes it, so `cargo test
--workspace` does not acquire a wasm-only cdylib), inherits nothing, and 2021 is
what published Zed extensions use.

## Verification, and where it lives

- **Static** (`cargo xtask config-check`): `[language_servers.wolf]` exists and
  lists `languages = ["Wolf"]` and not `"Wolfi"`; there is no live
  `[grammars.*]` table; `path_suffixes` is `["lu"]`; there is no live `grammar`
  key and no `block_comment`; `tab_size` is 4; and — the check nothing in Zed
  performs — the three spellings of `wolf` agree, i.e. the manifest table key,
  `SERVER_ID` in `src/lib.rs` and the settings id a user types are one string.
  A rename of the manifest key would otherwise silently make
  `LspSettings::for_worktree` read nobody's settings.
- **Build** (CI, `editor-matrix` job): `rustup target add wasm32-wasip2` then
  `cargo build --target wasm32-wasip2 --manifest-path clients/zed/Cargo.toml`.
  CI **builds** the wasm; CI does **not** run Zed.
- **Human** (`docs/MATRIX.md`): the row carries a stamp, and until someone
  stamps it the row reads as unverified.

```sh
cargo xtask config-check
cd clients/zed && cargo check                      # host: type-correct API usage
cargo build --target wasm32-wasip2                 # the real artifact
```

The static lane was exercised red before being trusted: adding a live
`[grammars.wolf]` table turns `config-check` red with the install-failure
reason, and removing it turns it green again.
