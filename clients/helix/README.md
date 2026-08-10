# helix

**Tier 2 — the config tier.** No plugin, no extension, no code: helix
configures a language server in TOML, so the whole client is
[`languages.toml`](languages.toml) and the interesting content of this
directory is what that file *cannot* do.

- Upstream: `helix-editor/helix`, read at `25.07.1` (Arch Linux
  `extra/helix 25.07.1-2`)
- Capability profile: [`profiles/helix.json`](../../profiles/helix.json)
- Recorded session: [`transcripts/helix/smoke.jsonl`](../../transcripts/helix/smoke.jsonl)

## What is in here

```
languages.toml            the whole client: two languages, one server
```

That is not a placeholder. `cargo xtask config-check` reads this file,
`cargo xtask helix-health` feeds it to helix itself, and
`transcripts/helix/smoke.jsonl` is a real session recorded through it.

## Setup

`wolf lsp` **is** the compiler (D34), so there is no server to install and no
version to keep in sync with anything.

1. Put `wolf` on `PATH`.
2. Append [`languages.toml`](languages.toml) to your own
   `~/.config/helix/languages.toml`.
3. Open a `.lu` file.

```sh
cat clients/helix/languages.toml >> ~/.config/helix/languages.toml
hx --health wolf     # should list `wolf` under "Configured language servers"
```

helix merges its `languages.toml` over the built-in defaults key by key, so
appending is the correct install and there is nothing to overwrite. If `wolf` is
not on `PATH`, change `command` to an absolute path — helix has no
`serverPath`-style setting and no auto-download, and neither does this file.

**The binary is called `hx` upstream.** Arch Linux installs it as
`/usr/bin/helix` and ships no `hx`; Homebrew and the release tarballs ship `hx`.
`cargo xtask helix-health` tries both rather than declaring one correct, and
this line is here because "command not found: hx" is otherwise a confusing first
five minutes.

## What works

Proven in [`transcripts/helix/smoke.jsonl`](../../transcripts/helix/smoke.jsonl),
a real 17-record session driven through a pty:

| feature | how | notes |
|---------|-----|-------|
| diagnostics | gutter, `Space d` | push, on open and on change |
| hover | `Space k` | `who: str`, range exact |
| document symbols | `Space s` | `main` |
| formatting | `:format`, format-on-save | canonical bytes round-trip to zero edits |
| code actions | `Space a` | wolf's fix-its arrive fully resolved |
| comment toggle | `Ctrl-c` | `//` only — wolf has no block comment form |
| syntax highlighting | — | **none.** See below |

## Known limitations — stated honestly

None of these is worked around here (D22: the editor layer must not launder what
the compiler said).

**A `.lu` buffer in helix has no syntax highlighting at all, and there is
nothing to ship in its place.** helix highlights exclusively through
tree-sitter — there is no regex fallback of the kind `clients/nvim/syntax/`
and `clients/vscode/syntaxes/` provide — and
`tenseleyFlow/tree-sitter-wolf` is a seed commit containing three files, none of
them `grammar.js` (`b1b2c17`, "scaffold; grammar port to follow
opportunistically"). So the `[[grammar]]` block ships **commented out**, with
that reason on the line above it. A config that references a missing grammar
makes `hx -g fetch` fail and `hx` noisy at startup, and noisy startup is how
users delete config.

Everything a *server* provides — diagnostics, hover, symbols, formatting, code
actions — works today and is what the transcript shows. What is missing is
strictly the local tokenizer.

**`hx --health` reports a tree-sitter parser that does not exist.** Pointing the
fragment at `grammar = "definitely-not-a-real-grammar"` still prints
`Tree-sitter parser: ✓`; only the `Highlight queries:` line goes `✘`. So the
health check cannot be used to detect a missing grammar, and
`cargo xtask helix-health` asserts on the highlight line and deliberately
ignores the parser line. This is a helix 25.07.1 behaviour, not a wolf one, and
it is written down because a check that appears to cover more than it does is
worse than one that covers less.

**`hx --health` always exits 0.** An unknown language, a server missing from
`PATH`, a fragment that failed to parse: all of them exit 0 and say so only on
stdout. Any CI assertion has to read the text, which is why this repo's lane
lives in `cargo xtask helix-health` rather than in a one-line workflow step.

**`.wolfi` is its own language with no server attached.** `wolfi` v0 is a
*binary* format — magic bytes `WOLFI`,
`upstream/crates/wolf_sema/src/interface.rs` — and `wolf lsp` discovers modules
by `.lu` alone (D32), so a `.wolfi` overlay has nothing for the server to
publish about. The `wolfi` block exists so the extension is *recognised* and so
that recognising it cannot be mistaken for serving it. Same ruling as ls04 and
ls05, and `hx --health wolfi` asserts it: `Configured language servers: None`.

**No `formatter` key, deliberately.** helix prefers a configured external
formatter **over** the language server, so
`formatter = { command = "wolf", args = ["fmt", "-"] }` would silently take
every format-on-save off `textDocument/formatting` — the path the transcripts
cover — and onto a second, untested spawn of the same binary. One formatting
path. `cargo xtask config-check` fails if that key appears.

**No `wolf.pkg` / `wolf.sum` language.** `clients/nvim` and `clients/vscode`
give the manifest files a filetype because both editors can highlight them
without tree-sitter. helix cannot, so a third language here would carry a
comment token and an indent width and nothing else. It can be added the day the
grammar lands.

**helix never sends `shutdown` or `exit`.** Verified across `:q`, `:qa` and
`:q!` — it drops the server process instead. That is a constraint on the server,
not a bug in this config, and it is filed in
[`docs/SERVER-CONSTRAINTS.md`](../../docs/SERVER-CONSTRAINTS.md).

**Only linux was exercised locally.** `cargo xtask helix-health` self-checks
that helix actually loaded the fragment from `$XDG_CONFIG_HOME` and **skips
loudly** where it did not, so a platform on which helix resolves its config
directory differently reports a skip rather than a vacuous pass.

## Verification, and where it lives

- **Static** (`cargo xtask config-check`): the server command is `wolf` and its
  args are `["lsp"]`, `file-types` is `["lu"]`, the `wolfi` block declares no
  server, there is no `formatter`, there is no live `[[grammar]]`, and the
  indent/width numbers agree with the other four clients. Needs no helix.
- **Real** (`cargo xtask helix-health`): drops this file into a temp config dir
  and asks **helix** whether it parsed — `--health wolf` recognises the language
  and configures `wolf`; `--health wolfi` recognises the language and configures
  nothing. Skips loudly with exit 77 when no `hx`/`helix` is installed.
- **In the harness**: the profile validates, the transcript replays,
  `lspconf onetruth` runs all 10 samples **under the helix profile** as one of
  nine, and `lspconf fuzz --profile=helix` puts a long edit session through this
  client's shape.

```sh
cargo xtask config-check
cargo xtask helix-health
```

Both were exercised red before being trusted: a fragment with a syntax error
turns `helix-health` red (4 problems, exit 1) and reverting it turns it green
again; adding `language-servers` to the `wolfi` block turns both lanes red.

## The encoding

**helix declares `["utf-8", "utf-32", "utf-16"]` and wolf answers `utf-8`.**
Note the order: helix offers utf-32 *second*, where Neovim offers it last. Since
wolf's own preference is utf-8 → utf-16 → utf-32 and utf-8 is first in both
lists, the two clients agree today for a reason that has nothing to do with
their own orderings — and a change to wolf's preference would move both at once.
`profiles/helix.json`'s `expects_encoding` is the independent statement that
makes that change fail the suite.

## Recording the transcript

helix resolves its server by bare `PATH` lookup, so a script named `wolf`
earlier on `PATH` captures everything with no instrumented build and no config
change:

```sh
# $SHIM/wolf, chmod +x
#!/bin/sh
if [ "$1" = "lsp" ]; then
  cd "$WOLF_LSP_ROOT/vendor/upstream/samples" || exit 1
  exec "$WOLF_LSP_ROOT/target/debug/lspconf" capture \
    --name helix/smoke --profile helix --workspace vendor/upstream/samples \
    -- "$WOLF_REAL" lsp
fi
exec "$WOLF_REAL" "$@"
```

helix is a TUI, so it is driven through a pty — the same shape ls03 used for
facsimile, with Python's stdlib `pty` module and no `pexpect`. Opening
`hello.lu` in `vendor/upstream/samples`, the session runs:

| keys | what it produces |
|------|------------------|
| *(startup)* | `initialize`, `initialized`, `didOpen`, first publish |
| `/who␍` `Esc` | cursor onto `who` |
| `Space k` | `textDocument/hover` |
| `Space s` | `textDocument/documentSymbol` |
| `Space a` | `textDocument/codeAction` |
| `i` `x` `Esc` | `didChange`, republish |
| `u` | `didChange`, republish clean |
| `:format␍` | `textDocument/formatting` |
| `:q!␍` | *(nothing — see below)* |

**The pty must be given a window size.** A pty from `pty.fork()` is 0×0, helix
lays its UI out from that, and it panics inside its own prompt
(`helix-term/src/ui/prompt.rs`, `Option::unwrap()` on `None`) the moment `:` is
typed — which looks exactly like a wolf failure and is not one. `TIOCSWINSZ` to
something like 120×40 before writing any keys is mandatory.

The driver script is not committed: it is scaffolding, and the transcript is the
artifact. The recipe above plus the key table reproduce it.

There is **no `.lsps` beside the transcript**, and that is the point — no script
decided what helix sent. `lspconf verify` knows the shape and
`lspconf replay transcripts/helix/smoke.jsonl` runs it against a live server (8
of the 17 records are deterministically matchable).
