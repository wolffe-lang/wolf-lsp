# What `wolf lsp` may not assume about real clients

Notes against the compiler track (s52 and after), discovered by integrating
actual editors rather than by reading the specification. Each one is a promise
the server has to keep because a client this repo tracks would otherwise break;
each names the client that proves it, so a promise can be retired when the last
client needing it is gone.

This file grows one client at a time (ls02–ls06). Nothing here is a request for
a new capability — it is the list of things that must stay true.

## From fackr (ls02, `496c7e2`)

**Publish diagnostics on open and on change — never only on save.** fackr
advertises `synchronization.didSave: true` and then never sends `didSave`
(`document_saved` exists with zero call sites). A server that waited for a save
would look completely dead in this editor. *Holds today.*

**Accept full-text `didChange` regardless of what `textDocumentSync`
advertises.** Every change fackr sends is the whole buffer in a single
`contentChanges: [{text}]`, found by hashing on a ~50 ms tick, and it never
reads the server's advertised sync mode. wolf advertises `change: 1` (Full), so
the two agree — by luck, not negotiation. *Holds today; asserted by
`lspconf fuzz --profile=fackr`.*

**Survive a `didChange` that arrives before `initialized`.** fackr queues only
`didOpen` while a server is `Initializing`; a change can overtake the
handshake. *Untested against wolf; no failure observed.*

**Survive a re-open of an already-open document that carries no content.**
`open_document` early-returns for a tracked path, so the server may hold an
overlay the editor believes it replaced.

**Keep stderr quiet in normal operation.** fackr pipes stderr; before this
sprint's patch it never read it, and a server would deadlock at ~64 KiB of
logging. The patch drains it, but every unpatched fackr in the world still has
the old behaviour, and so do other hand-rolled clients. *Holds today — `wolf
lsp` writes nothing to stderr in a clean session.*

**Answer `utf-32` when it is the only offered encoding.** fackr's columns are
`ropey` char offsets, and it declares `["utf-32"]` alone for exactly that
reason. wolf's preference order is utf-8 → utf-16 → utf-32, so utf-32 is
reachable only as a sole offer; a change to that order would silently corrupt
every column past an astral character in this editor. *Holds today; asserted by
`profiles/fackr.json` and by fackr's own live test.*

**Front-load the essential sentence of a diagnostic into the first line of
`message`, and keep `range.start.line` exact.** fackr renders a diagnostic as
one coloured dot in the gutter at `range.start.line` and displays no text at
all. Until that is fixed upstream, the line number is the entire user-visible
signal. *Holds today.*

**Echo URIs back byte-identically.** fackr compares URIs by string equality and
never percent-decodes; it also builds `rootUri` by concatenation, so a path with
a space or a `#` produces an invalid URI the server must not normalize into
something else. *Holds today.*

**Integer request ids only, and never a response before the initialize
response.** fackr parses only integer ids (`as_i64`) and detects the initialize
response *without checking its id* — the first response it sees while
`Initializing` is treated as the handshake. A server that answered anything
before `initialize` would be misread as the server's capabilities. *Holds
today.*

**Do not block on a server→client request.** `workspace/configuration` is
always answered `[]`, `client/registerCapability` is acked and discarded, and
`workspace/applyEdit` is refused `-32601` despite being advertised. There is no
channel for server-side settings in this client at all. *Holds today — wolf
sends no server→client requests.*

**Tolerate a SIGKILL microseconds after `exit`.** fackr sends `shutdown`,
sleeps 100 ms, sends `exit`, and kills the process. *Holds today.*
