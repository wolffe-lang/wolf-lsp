# yew

**Tier 3, the documented tier: a recipe and one config file.** yew
(`tenseleyFlow/sagitta`) is a C11 modal terminal editor with a real LSP client
(`src/mod/lsp/`). Its server table is compiled in and has **no wolf row**, so
opening a `.lu` file starts no server unless your config adds one. This
directory has that row. It does not patch yew: yew belongs to another org, and
the fix on yew's side is a proposed patch that its maintainers can take or
leave (see *Upstream* below).

## Why a `.lu` file gets no server (measured, ye01)

yew starts a server for a buffer only if the buffer's language has a row in
its server table. The language comes from yew's own syntax pack:
`runtime/syntax/wolf.fl` maps `lu` (and `wolfi`) to the language `wolf`. The
compiled-in table (`default_cfgs` in `src/mod/lsp/client.c`, sagitta
`d52f24d5`) lists clangd, rust-analyzer, pyright, gopls, tsserver, fortls and
bashls, and nothing for `wolf`. A missing row is not an error in yew, so
nothing is logged and nothing is spawned. The only visible sign is the reply
to `:ed.lsp.info`:

```
no LSP server configured for wolf
```

Measured through a pty with an empty `XDG_CONFIG_HOME`, using both the
installed `yew 1.0.0-dev` and a build of `d52f24d5`. Neither has a child
process besides yew's own helper, and neither writes an LSP line to its log.
yew's own unit suite enforces this: `test_lsp_config.c` asserts that
`yew_lsp_client_cfg(ed, "wolf")` is NULL with the default config. The server
is not involved. Once the row is present, `wolf lsp` starts and the session is
clean (below).

## Install

Append [`init.fl`](init.fl) to `$XDG_CONFIG_HOME/yew/init.fl` (usually
`~/.config/yew/init.fl`):

```fletch
let lsp = {servers: {
    wolf: {id: "wolf", cmd: "wolf", args: ["lsp"], roots: ["wolf.pkg", ".git"]},
}}
```

**An `lsp.servers` table replaces yew's defaults. It does not merge with
them.** yew reads the whole `lsp.servers` map in place of its compiled-in table
(`client_cfg_load` sets `replaces_defaults`). If this is your only row, clangd,
rust-analyzer and the others stop starting. Copy in the rows you use.

`wolf` must be on the `PATH` yew inherits, because yew looks up `cmd` there.
You can set `cmd` to an absolute path instead.

## What the session looks like

Recorded with wolf 0.2.23 (the darwin arm64 release archive, sha256
`92c918f2…`), using a `tee` shim on the server's stdio:

| direction | message |
|---|---|
| c→s | `initialize`: `positionEncodings: ["utf-8","utf-16"]`, `clientInfo: yew 1.0.0-dev`, `rootUri` set to the `wolf.pkg`/`.git` root, or to yew's workspace root if neither is found |
| s→c | result: `positionEncoding: "utf-8"`, `serverInfo: wolf-lsp 0.2.23` |
| c→s | `initialized`, then `textDocument/didOpen` with `languageId: "wolf"` |
| s→c | `textDocument/publishDiagnostics`, for example E0401 at line 1, characters 17–24 (0-based) for a `str` bound to an `int` |
| c→s | `shutdown`, `exit` on quit |

`:ed.lsp.info` prints `wolf ready; root …; utf-8; caps completion,hover,…`.
`:ed.lsp.diagnostics` lists the diagnostic as ``hello.lu:2:18 ✗ this is
`str`, but `n` is …``. yew's column is 1-based, and 18 is UTF-8 offset 17.

**Encoding.** yew offers `["utf-8","utf-16"]` in that order and wolf answers
`utf-8`. This is the same branch Neovim and helix reach. yew has no
`utf-32`, so fackr's branch is not one it can take.

**Capabilities yew does not ask for** (from yew's s46/s47 contracts, not from
this capture): `snippetSupport: false`, `documentationFormat: ["plaintext"]`,
`workspace.applyEdit: false`, and no `resourceOperations`. None of these
conflict with `docs/SERVER-CONSTRAINTS.md`. Because of `applyEdit: false`,
wolf must not send `workspace/applyEdit`. That is already the server's rule
for v0.

## Upstream

The fix that makes this file unnecessary is a single row in yew's
`default_cfgs`, `{"wolf", "wolf", "wolf", {"lsp"}, {"wolf.pkg", ".git"}}`,
together with the change to `test_lsp_config.c` that asserts it. ye01 wrote
that patch against `d52f24d5` and ran yew's unit suite on it (3014 tests, 0
failures). Restoring yew's `client.c` while keeping the new test made exactly
that test fail. **It is not submitted.** yew belongs to another org, and
whether to carry it is up to its maintainer.

## Verification

None in CI: no runner has yew installed. `cargo xtask config-check` checks the
config statically. It requires `init.fl` to spawn `wolf lsp`, to put
`wolf.pkg` before `.git`, and to give `wolfi` no server, and it requires this
README to quote `init.fl`'s `let lsp` block word for word. A person walks the
recipe once per release and stamps the row in `docs/MATRIX.md`.
