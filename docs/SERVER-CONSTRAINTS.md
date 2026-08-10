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

## From facsimile (ls03, `1242ffa`)

**Never require a response to a server→client request — and at v0, send none
at all.** `handle_request` in facsimile is an empty stub: its whole body is a
`TODO` and an `if (.false.) print *, …` that exists to silence unused-argument
warnings. `workspace/configuration`, `client/registerCapability` and
`window/workDoneProgress/create` get **no reply of any kind, ever**. A server
that blocks waiting for one hangs this editor forever, with no error, no
timeout and no log line — the user sees an editor that has simply stopped.

*Holds today, structurally:* `wolf lsp` constructs only `Message::Response` and
`Message::Notification` and never `Message::Request`
(`crates/wolf_lsp/src/server.rs`; its inbound `Message::Response(_)` arm is
commented "we send no server→client requests"). Proven empirically by
`transcripts/facsimile/smoke.jsonl` — 15 records, zero server→client requests,
a session that completes normally with hover, symbols, formatting and
diagnostics all delivered.

**This is a standing constraint on every future capability, not a one-time
check.** Anything that would need `workspace/configuration` (user settings),
`client/registerCapability` (dynamic registration) or
`window/workDoneProgress/create` (progress reporting) must be **opt-in, gated
on a client capability, and degrade to a working default when the reply never
comes.** "Send it and wait" is not available to this server while facsimile is
tier 0. A timeout is the minimum bar and is still worse than not sending.

**Emit raw UTF-8 in JSON strings — never `\uXXXX` escapes.** facsimile's
hand-rolled `json_module` does not decode escape sequences in either
direction, so an escaped character arrives at the user as the literal six
characters. serde_json does the right thing by default, which is exactly why it
is pinned rather than trusted: *Holds today; asserted by
`tests/encoding.rs::the_server_emits_raw_utf8_and_never_backslash_u_escapes`,
which reads raw frame bytes rather than parsed JSON, because parsing is what
would hide the bug.*

**Integer request ids only, and small ones.** All numbers in facsimile's JSON
are `real64`, so ids round-trip through a double (bounded by 2^53) and
**string ids are unsupported entirely**. Echo ids back exactly as received.
*Holds today.*

**Answer `utf-16` when it is the only offered encoding.** facsimile's columns
are UTF-16 code units, converted correctly at every site — including non-BMP —
by `utf8_char_col_to_utf16` / `utf16_to_utf8_char_col`. It now declares
`general.positionEncodings: ["utf-16"]` alone. wolf's preference order is
utf-8 → utf-16 → utf-32, so a sole offer is the only way to reach utf-16; a
change to that order would silently shift every column past a multi-byte
character. Note this is the exact mirror of fackr's constraint, and the two
together pin both ends of the preference order. *Holds today; asserted by
`profiles/facsimile.json`, by `lspconf onetruth` under that profile, and by the
negotiation recorded in the transcript.*

**Publish diagnostics on open and on debounced change — never only on save.**
facsimile declares no `synchronization` object at all and sends no `didSave`
in the recorded session. It also declares no `publishDiagnostics` capability
while handling diagnostics anyway. A server that keyed diagnostics off a
declared capability, or off `didSave`, would look dead here. *Holds today.*

**Accept full-text `didChange` regardless of what `textDocumentSync`
advertises, and expect no useful version numbers.** Every change is the whole
buffer in one `contentChanges: [{text}]` on a 0.5 s debounce, and the
`version` field is **hardcoded to 1** — so it carries no ordering information
whatsoever. The server may not use client versions to order or discard edits.
*Holds today; asserted by `lspconf fuzz --profile=facsimile --splices=200`.*

**Survive a client that never closes a document.** `notify_file_closed` exists,
is exported, is imported by two modules, and has **zero call sites**. Open
documents accumulate for the life of the session, so the server's overlay set
only ever grows. *Holds today.*

**Survive SIGTERM with no `shutdown`/`exit` handshake, leaving no orphan.**
facsimile sends neither message: it SIGTERMs the server and SIGKILLs it 100 ms
later. This is why the recorded transcript ends at the last response rather
than at an exit. It is also why the pin's new "bare `exit` exits 1" behaviour
is invisible to this client — it never sends `exit` at all. *Holds today;
asserted by `tests/semantics.rs::a_client_that_vanishes_leaves_no_orphan`.*

**Keep responses small.** facsimile's JSON parse and serialize are both O(n²)
in message size and its `read_buffer` regrows by copy. A large completion list
or a long markdown hover visibly stalls the editor. *Holds today — wolf's
hover is a short code fence.*

**Two response shapes, for capabilities wolf does not serve yet.** When
completion lands it must be a `CompletionList` (`{"items": […]}`) — a bare
array yields zero items — and definitions must be `Location`/`Location[]`,
because `LocationLink[]` is not parsed **despite `linkSupport: true` being
advertised**. *Not yet applicable; recorded so the sprint that adds them does
not have to rediscover it.*
