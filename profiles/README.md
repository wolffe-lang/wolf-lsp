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
| `helix`     | ls06    | owed |
| `zed`       | ls06    | owed |

Marking one of those `synthetic` to make the list shorter does not work:
`missing_derived` keys off the provenance, not the filename.

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
