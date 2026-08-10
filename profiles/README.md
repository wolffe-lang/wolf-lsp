# Capability profiles

One client-capability document per client, plus the synthetic documents the
protocol cases need. `lspconf profiles` validates every one and prints which
real clients still have none.

## Two kinds, and the file says which

- **`synthetic`** — a document that claims to be nobody, with a stated purpose.
  `minimal` (declares nothing optional) and `maximal` (declares everything)
  bracket every real client between two documents that exist today; the
  `utf8-first` / `utf16-only` / `utf32-only` / `unknown-encoding` set reaches
  all three negotiated encodings and the spec-mandated fallback, which is what
  the ls01 §5 suite runs against.
- **`derived`** — read off a real client, recording the repository, the commit,
  the date, and the files it was read from. Validation refuses a `derived`
  profile missing any of those, because that is exactly the shape a fiction
  would take.

A profile invented here rather than read off a client is a lie the suite then
tests against, which is worse than having no profile: it produces a green lane
for a client nobody checked. So the six real clients ls01 §4 names start with **no
profile**, and `lspconf profiles` says so on every run, naming the sprint
that owes each:

| client      | owed by | state |
|-------------|---------|-------|
| `fackr`     | ls02    | **derived** — read off `fackr@496c7e2` and the session it recorded |
| `facsimile` | ls03    | **derived** — read off `facsimile@1242ffa` and the session it recorded |
| `nvim`      | ls04    | **derived** — read off `neovim@v0.12.4` and the session it recorded |
| `vscode`    | ls05    | **derived** — read off `vscode@df53daa` (1.132.0) and the session it recorded |
| `helix`     | ls06    | **derived** — read off `helix@25.07.1` and the session it recorded |
| `emacs`     | ls06    | **derived** — read off `emacs@30.2` (eglot 1.17.30) and the session it recorded |
| `zed`       | ls06    | owed |

Marking one of those `synthetic` to make the list shorter does not work:
`missing_derived` keys off the provenance, not the filename.

`emacs` is a **seventh** row, added by ls06 where ls01 §4 named six. eglot turned
out to be drivable under `emacs --batch`, so there is a real session to read a
profile off — and a tracked client whose profile nothing watches for staleness is
exactly the gap this list exists to close. Membership is a claim about
*tracking*, not about tier; `docs/MATRIX.md` is where tiers live.

**`zed` is the one row still owed, and it is owed for a reason no amount of
effort in this sprint could remove.** Zed's dev-extension install is a GUI action
(`zed::InstallDevExtension`); its CLI has no `--install-extension` and no
`--dev-extension` flag, and `auto_install_extensions` covers *published*
extensions by id rather than dev extensions. There is no headless way to load the
extension, so there is no headless way to record what Zed's client sends. The
slot stays empty until somebody runs Zed on a desktop with the capture shim on
`PATH`.

`helix`'s and `emacs`'s `commit` fields are release versions for the same reason
`nvim`'s is, and with the same honesty caveat: both were read off distribution
packages (`extra/helix 25.07.1-2`, `extra/emacs-nox 30.2-3`) that publish no
build sha, and the `source` list carries the full version banner so the exact
build is identifiable. What the sprint requires is that the profile be stamped
with the version it was read from — a client's declared capabilities move between
releases and a stale profile is a lie — and a version satisfies that directly.

`nvim`'s `commit` is a **release tag**, not a sha, and that is the honest
recording rather than a shortcut: `nvim --version` publishes no build sha, and
the machine it was read on installed a distribution package. `v0.12.4` names a
revision unambiguously, and the `source` list carries the full version banner
(`NVIM v0.12.4`, RelWithDebInfo, LuaJIT 2.1.1785192264) so the exact build is
identifiable. The sprint's requirement is that the profile be stamped with the
Neovim version it was read from — Neovim's declared client capabilities move
between releases and a stale profile is a lie — and a tag satisfies that more
directly than a sha would.

A derived profile is not decoration. `lspconf onetruth` runs every sample under
the three synthetic encoding documents **plus every derived one**, so a real
client's capability set is part of the claim that `wolf build` and `wolf lsp`
agree — and `lspconf fuzz --profile=<client>` puts a long edit session through
that client's shape.

## `expects_encoding`

Every profile states the encoding it expects to negotiate. That is an
independent statement, not a reading of the server — asserting the server
agrees with itself proves nothing. Writing the expectation down means a change
to the negotiation *rule* fails the suite even when the server stays
self-consistent, which is the only way that rule is testable at all.

VS Code is where that statement earns the most. `vscode-languageclient` declares
`["utf-16"]` and then *throws* if the server names anything else
(`lib/common/client.js:835`), so `expects_encoding: "utf-16"` in
`profiles/vscode.json` is not a prediction about rendering — it is the
difference between an extension that starts and one that does not. A reordering
of wolf's preference that still produced self-consistent utf-8 would sail past
every server-side assertion and break this client outright.
