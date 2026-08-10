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
for a client nobody checked. So the six real clients ls01 §4 names have **no
profile yet**, and `lspconf profiles` says so on every run, naming the sprint
that owes each:

| client      | owed by |
|-------------|---------|
| `fackr`     | ls02    |
| `facsimile` | ls03    |
| `nvim`      | ls04    |
| `vscode`    | ls05    |
| `helix`     | ls06    |
| `zed`       | ls06    |

Marking one of those `synthetic` to make the list shorter does not work:
`missing_derived` keys off the provenance, not the filename.

## `expects_encoding`

Every profile states the encoding it expects to negotiate. That is an
independent statement, not a reading of the server — asserting the server
agrees with itself proves nothing. Writing the expectation down means a change
to the negotiation *rule* fails the suite even when the server stays
self-consistent, which is the only way that rule is testable at all.
