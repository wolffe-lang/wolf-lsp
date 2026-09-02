# vscode

**Tier 1 — the plugin tier.** Like `nvim/`, this directory holds real code: it
is the root of a VS Code extension, and it ships from here until extension
distribution is decided (ls07). So `clients/vscode/` is simultaneously the
deliverable, the mirror and the documentation.

The extension is boring on purpose. It registers two languages, starts `wolf
lsp`, and gets out of the way (D34). Everything interesting a user sees in a
`.lu` buffer — diagnostics, hover, document symbols, formatting, code actions —
arrives because `vscode-languageclient` registered a provider for a capability
the *server* advertised. `src/extension.ts` contains no provider, no diagnostic
collection, and no middleware, and that is a D22 requirement rather than a
stylistic one: a client that rewrites a message or remaps a severity becomes a
second, unreviewed authority on what the compiler said. The way to not be that
is to have nowhere to put the code.

- Upstream: `microsoft/vscode`, read at `1.132.0`
  (`df53daabb18cd157bdb08c7f01c34df936cf12f4`, stable, linux-x64)
- Client library: `vscode-languageclient` 9.0.1
- Capability profile: [`profiles/vscode.json`](../../profiles/vscode.json)
- Recorded session: [`transcripts/vscode/smoke.jsonl`](../../transcripts/vscode/smoke.jsonl)
- Token table provenance: [`inventory.md`](inventory.md)

## What is in here

```
package.json                      contributions: languages, grammars, settings, commands
language-configuration.json       comments, brackets, indentation — read off wolf_fmt
syntaxes/wolf.tmLanguage.json     GENERATED from the pinned grammar
syntaxes/wolfi.tmLanguage.json    GENERATED — `source.wolf` and nothing else
syntaxes/wolf-pkg.tmLanguage.json GENERATED — likewise
src/extension.ts                  the whole client: ~200 lines, most of them comments
src/pin.ts                        GENERATED from vendor/upstream/PIN
src/test/grammar.ts               the grammar lane — no VS Code, no server
src/test/runTest.ts               launches headless VS Code
src/test/suite/                   the extension lane and the server lane
src/test/snapshots/               reviewed scope-name snapshots
.vscodeignore                     what must not reach a user's machine
```

Four of those are generated and **must not be hand-edited**: `cargo xtask
grammar-drift` regenerates them from the pin and compares bytes, so an edit
shows up as drift. See [`inventory.md`](inventory.md).

## Setup

`wolf lsp` **is** the compiler (D34), so there is no server to install and no
version to keep in sync with anything.

1. Put `wolf` on `PATH`.
2. Install the extension.
3. Open a `.lu` file.

If `wolf` is not on `PATH`, set `wolf.serverPath`. There is no third option and
no auto-download.

### Installing, which today means a vsix

There is **no marketplace listing and no Open VSX listing** — ls07 owns
publisher identity, tokens and release channels. Until then the vsix is the
install path, and it is a first-class one rather than a developer footnote:

```sh
cd clients/vscode
npm ci
npm run package                       # -> wolf-0.0.1.vsix
code --install-extension wolf-0.0.1.vsix
```

VSCodium, Cursor and any other VS Code distribution take the same file through
their own CLI:

```sh
codium --install-extension wolf-0.0.1.vsix
cursor --install-extension wolf-0.0.1.vsix
```

Those are the commands CI runs, and `npm run package` is a plain `vsce package`
— no wrapper script deciding anything.

**The `publisher` field says `wolf-lang-unpublished`, deliberately.** `vsce`
refuses to package without one, and inventing a plausible-looking publisher name
here would either squat an identity we do not own or bake a wrong one into every
install instruction downstream. It is a placeholder, it is not registered, and
ls07 replaces it with the real identity in the same commit that first publishes.
Local vsix installs do not care what it says.

### Developing against it

```sh
npm ci && npm run compile
code --extensionDevelopmentPath="$PWD" ../../vendor/upstream/samples
```

## What works

Proven in [`transcripts/vscode/smoke.jsonl`](../../transcripts/vscode/smoke.jsonl),
a real 42-record session, with every assertion checked as it was recorded:

| feature | how | notes |
|---------|-----|-------|
| diagnostics | — | push, on open and on change |
| hover | hover, `K` | `who: str`, range exact |
| document symbols | outline, breadcrumbs, `Ctrl+Shift+O` | `main` |
| formatting | `Shift+Alt+F`, `editor.formatOnSave` | canonical bytes round-trip to zero edits |
| code actions | the lightbulb, `Ctrl+.` | wolf's fix-its arrive fully resolved |
| syntax highlighting | — | independent of the server; see `inventory.md` |
| comment toggle | `Ctrl+/` | `//` only — wolf has no block comment form |
| problem matcher | `$wolf` in a task | rustc-shaped output → the Problems panel |

Note what the transcript shows that no one wrote down: **VS Code polls
`textDocument/codeAction` constantly** — on every cursor move, for the lightbulb
— and requests `documentSymbol` on its own for breadcrumbs and the outline. 24
of the 42 records are traffic the extension never asked for. That is a server
constraint discovered by recording rather than by reading, and it is filed in
[`docs/SERVER-CONSTRAINTS.md`](../../docs/SERVER-CONSTRAINTS.md).

## The encoding, which is not negotiable from this client

**VS Code and wolf settle on `utf-16`, and VS Code will accept nothing else.**

`vscode-languageclient` 9.0.1 declares `general.positionEncodings: ["utf-16"]`
— hardcoded at `lib/common/client.js:1370`, with no option to change it — and
then *throws* if the server's `InitializeResult` names any other encoding
(`client.js:835`):

```js
if (result.capabilities.positionEncoding !== undefined &&
    result.capabilities.positionEncoding !== PositionEncodingKind.UTF16) {
  throw new Error(`Unsupported position encoding (…) received from server ${this.name}`);
}
```

So this client is not merely a utf-16 client the way facsimile is; it is one for
which a wrong answer is a **hard client-side failure**, not a mismatch anybody
gets to debug from the wire. The extension does not trim, extend, or override
any of that — the recorded `initialize` is the whole claim, and
`profiles/vscode.json` is read off it.

## Known limitations — stated honestly

None of these is worked around here (D22: the editor layer must not launder what
the compiler said).

**`.wolfi` gets a language and highlighting, but no server — and this is a
delta from the sprint text.** ls05 §1 asks for the language `wolf` to carry
`.lu` *and* `.wolfi`, which would put both under one `documentSelector`. Two
facts at this pin argue against it. First, `wolf lsp` has no `.wolfi` path:
module discovery is `.lu`-keyed (D32 makes every `.lu` in a directory one
module), so a `.wolfi` overlay has nothing to publish about. Second, and more
decisively, **`wolfi` v0 is a *binary* format** — magic bytes `WOLFI`,
`upstream/crates/wolf_sema/src/interface.rs` — and nothing in the tree writes a
file with that extension at all; `wolf interface` pretty-prints the bytes for
humans instead. So `.wolfi` is its own language id with the same grammar and no
client attached, which is exactly the shape ls04 settled on for the same reason:
attaching to documents the server ignores produces a buffer that looks supported
and is not. If `.wolfi` becomes a text artifact upstream, this is a one-line
change and a test already asserts the current shape.

**Thirteen operators do not highlight, and `==` highlights wrongly.** The
vendored `grammar.ebnf` elides the precedence climb, so the generated operator
inventory has no `==` `!=` `<` `>` `<=` `>=` `<=>` `&&` `||` `<<` `>>` `/` `&`.
The visible consequence is in the committed snapshot: `strings.lu` line 33
records `==` as *two separate* `=` operator tokens and `&&` as unscoped text.
ls04 filled the same hole by hand from the un-vendored `spec/01-grammar.md`
§3.2; this sprint does not, because a generator that reaches past its stated
input is a generator whose output nobody can re-derive. See
[`inventory.md`](inventory.md) for the two upstream remedies.

**`wolf build` does not exist at this pin.** It lands at wolf-lang s31; the
binary answers every unknown subcommand with `wolf: pre-alpha scaffold; wolf
build|run lands at sprint s31`. The `$wolf` problem matcher and the `wolf` task
type ship anyway, and the matcher's two patterns are verified against real
output — from `wolf conform-run`, which renders through the same `wolf_diag`
renderer (`error[E0002]: …` then ` --> file:line:col`). Not shipping the matcher
would mean nothing picks the format up the day `build` lands; claiming it was
tested against `wolf build` would be a lie about which command produced the
bytes.

**Semantic tokens and inlay hints arrive over the protocol as of the `3befc3e`
pin (wolf-lang v0.2.3, s134), and this extension's part in each is different.**
Semantic tokens needed a manifest entry and now have one: VS Code's fallback
for a token type a theme has no rule for is *no rule*, so a served token type
with no `semanticTokenScopes` mapping would leave the file LESS coloured than
the TextMate grammar left it. `contributes.semanticTokenScopes` maps all eight
types in the server's closed legend — `namespace type parameter variable
property enumMember function keyword` — and the `declaration`/`readonly`
modifier selectors, each ending in a standard TextMate scope every theme
already styles. A test asserts the mapping covers the legend exactly, in both
directions. Inlay hints needed nothing: there is no contribution point for
them, and whether they show is the user's `editor.inlayHints.enabled`, which
this extension does not set.

**The "no toolchain" notification is not asserted by the harness.** VS Code
exposes no API for reading its own notifications, so no test can claim the
warning appeared. What *is* asserted, and what matters more, is the whole
artifact suite passing on a machine with no `wolf` on `PATH`: the extension
activates, highlights, registers its commands, and does not throw. The
notification path itself is eight lines with no branching beyond the button
choice.

**Tree-sitter is irrelevant here, and that is not a gap.** VS Code tokenizes
with TextMate regardless of what `tree-sitter-wolf` does; the tree-sitter work
benefits Neovim, Helix, Zed and linguist. Both grammars are downstream of the
same EBNF inventory, which is what keeps them from disagreeing.

**Only linux was exercised locally.** The three-OS matrix is CI's (D35). The
extension spawns through `child_process` with `windowsHide` and lets the OS
resolve `PATH`, and `runTest.ts` adds `--no-sandbox` only on linux — but "it is
written to be portable" is not "it was run on Windows", and this line is here so
nobody reads the CI matrix as a claim that predates its first green run.

## Recording the transcript

The extension resolves its server by bare `PATH` lookup, so a script named
`wolf` earlier on `PATH` captures everything with no instrumented build and no
extension change:

```sh
# $SHIM/wolf, chmod +x
#!/bin/sh
if [ "$1" = "lsp" ]; then
  cd "$WOLF_LSP_ROOT/vendor/upstream/samples" || exit 1
  exec "$WOLF_LSP_ROOT/target/debug/lspconf" capture \
    --name vscode/smoke --profile vscode --workspace vendor/upstream/samples \
    -- "$WOLF_REAL" lsp
fi
exec "$WOLF_REAL" "$@"
```

```sh
cd clients/vscode
WOLF_LSP_ROOT=/path/to/wolf-lsp \
WOLF_REAL=/path/to/wolf \
WOLF_BIN=$SHIM/wolf \
DISPLAY= xvfb-run -a node ./out/test/runTest.js
```

Two details that are not incidental:

**The shim passes every non-`lsp` subcommand through.** ls04's equivalent could
ignore its arguments, because Neovim only ever spawns `wolf lsp`. This extension
runs `wolf --version` first, as its discovery probe — a shim that swallowed that
would make the extension believe there is no toolchain and never start a server
to record.

**`runTest.ts` puts `dirname($WOLF_BIN)` on `PATH` rather than setting
`wolf.serverPath`.** That exercises the discovery path a real user has, and it
is what lets the shim work at all.

The recorded session is the extension test suite, so **every assertion runs
while the session is being recorded**. A transcript of a broken session is worse
than none, because it replays green forever.

There is **no `.lsps` beside it**, and that is the point — no script decided what
VS Code sent. `lspconf replay transcripts/vscode/smoke.jsonl` runs it against a
live server (18 of the 42 records are deterministically matchable).

## Verification, and where it lives

Three lanes, split by what each needs, so a failure names its own cause:

- **Grammar lane** (`npm run test:grammar`, 4 cases): tokenizes vendored corpus
  samples with `vscode-textmate` and `vscode-oniguruma` — the exact tokenizer
  and exact regex engine VS Code runs — and compares scope-name snapshots. Needs
  **no VS Code and no `wolf`**, so it runs anywhere, including in a container
  with no display.
- **Extension lane** (`npm run test:extension`, 8 cases): a real headless VS
  Code. Contributions, activation, language registration, the settings surface,
  the command set, and the absence of semantic-token UI. Needs **no `wolf`
  binary**, which is what keeps this lane from being dark on CI runners.
- **Server lane** (the same run, 6 more cases): diagnostics with the exact
  one-truth position, hover with an exact range, `documentSymbol`, a formatting
  round-trip, and a fully-resolved code action. Skips **loudly** with a reason
  when there is no `wolf` at the pin (ls00 §3).
- **In the harness**: the profile validates, the transcript replays, `lspconf
  onetruth` runs all 10 samples **under the vscode profile** as one of seven,
  and `lspconf fuzz --profile=vscode` puts a long edit session through this
  client's shape.
- **Derivation**: `cargo xtask grammar-drift` regenerates all four generated
  files from the pin and byte-compares.

```sh
cargo xtask grammar-drift
cd clients/vscode && npm ci && npm test          # grammar lane, then VS Code
```

Snapshots are reviewed like any other snapshot in this repository
(CONTRIBUTING.md, "the snapshot ritual"):

```sh
UPDATE_SNAPSHOTS=1 npm run test:grammar   # then READ the diff
```

## Why mocha here and no busted in `nvim/`

Consistency would have argued for a hand-rolled runner in both, and
`src/test/grammar.ts` is one — forty lines, one line per case, no framework.
The VS Code lane is different: `@vscode/test-electron` hands control to a
`run()` function inside a live editor process, and something has to own
asynchronous test scheduling, timeouts and reporting in there. Mocha is what
VS Code's own extension samples use, it is already the shape
`@vscode/test-cli` assumes, and writing a third scheduler to avoid one
devDependency would be the expensive kind of consistency.
