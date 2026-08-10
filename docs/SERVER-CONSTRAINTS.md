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

## From Neovim (ls04, `v0.12.4`)

Neovim is the first *well-behaved* client on this list — it answers
server→client requests, cancels properly, closes documents, and shuts down
cleanly — so it constrains the server in a different way than fackr and
facsimile do. What it pins is the **preference order**, not a workaround.

**The encoding preference order is user-visible, and Neovim is where a change
to it lands hardest.** Neovim declares
`general.positionEncodings: ["utf-8", "utf-16", "utf-32"]` — all three, utf-8
first — so wolf's own preference is the only thing deciding the wire format,
and today it decides `utf-8`. Reordering the server's preference (or dropping
utf-8) would silently change every Neovim user's positions with no client-side
signal at all: unlike fackr, Neovim converts correctly to whatever is
negotiated, so a *wrong* choice here is invisible rather than broken. That is
worse. *Holds today; asserted by `profiles/nvim.json`'s `expects_encoding` and
by a live assertion on `client.offset_encoding` in the recorded session.*

**Start, and diagnose, with no `rootUri`.** Neovim's native config has no
`single_file_support` field; a client whose `root_markers` match nothing gets a
nil root and starts anyway (`workspace_required` is unset). So a scratch `.lu`
in a directory with no `wolf.pkg` and no `.git` still expects diagnostics.
*Holds today.*

**Do not require `workspace/configuration`, and do not wait for settings.**
Neovim would answer, so this is not the hang facsimile would suffer — but the
plugin sends no `settings` block by design, and a server that treated absent
settings as "not ready" would stall a client that is behaving perfectly.
*Holds today — `wolf lsp` reads no settings.*

**Keep `textDocument/formatting` byte-stable on canonical input.** Neovim's
`gq` and `:WolfFmt` both route through `textDocument/formatting`, and the
recorded session asserts that formatting a corpus sample returns an **empty**
edit list. A response that returned a no-op edit instead of no edits would mark
every formatted buffer modified and burn an undo state per format. *Holds
today; asserted in `clients/nvim/tests/smoke.lua`.*

## From VS Code (ls05, `1.132.0`, vscode-languageclient 9.0.1)

VS Code is, like Neovim, a well-behaved client — but it is the first one whose
*client library* enforces a constraint the server can violate only once.

**Answer `utf-16`, or the client throws.** `vscode-languageclient` declares
`general.positionEncodings: ["utf-16"]` — hardcoded at
`lib/common/client.js:1370`, with no extension-facing option to change it — and
then refuses any other answer outright:

```js
if (result.capabilities.positionEncoding !== undefined &&
    result.capabilities.positionEncoding !== PositionEncodingKind.UTF16) {
  throw new Error(`Unsupported position encoding (…) received from server ${this.name}`);
}
```

Every other client on this list would *mis-render* a wrong encoding. This one
fails to start, in a `try`/`catch` that surfaces as a notification, and the user
gets an extension that does nothing. Note that this is the same wire outcome as
facsimile's constraint and a strictly stronger requirement: facsimile needs
utf-16 to be *reachable*, VS Code needs it to be *the answer*. *Holds today —
wolf's preference order is utf-8 → utf-16 → utf-32 and a sole utf-16 offer
selects utf-16; asserted by `profiles/vscode.json`, by `lspconf onetruth` under
that profile, and by the negotiation recorded in `transcripts/vscode/smoke.jsonl`.*

**Expect `textDocument/codeAction` on every cursor move, unprompted.** The
recorded 42-record session contains **nine** `codeAction` requests and **three**
`documentSymbol` requests; the suite that drove it issued exactly one of each.
The other eight and two are VS Code's own — it polls code actions to decide
whether to draw the lightbulb, and requests document symbols for breadcrumbs and
the outline. With their responses and the four `$/setTrace` notifications, **24
of the 42 records are traffic nobody asked for.** This is the highest request rate of
any client tracked so far, it scales with typing and cursor movement rather than
with anything the user asks for, and it arrives on *clean* files where there is
nothing to fix. A `codeAction` handler whose cost is proportional to anything
but the diagnostics already computed will be felt here first. *Holds today —
wolf resolves fix-its at publish time, so the response is a lookup.*

**Expect `$/setTrace` at any point, including several in a row.** The client
sends it on connection and whenever `wolf.trace.server` changes; the recorded
session carries four. A server that treated an unknown notification as an error
would be reacting to a setting the user changed in a different window. *Holds
today — unknown notifications are ignored.*

**`shutdown` then `exit`, and the client waits for the response.** Unlike
facsimile (which SIGTERMs) and fackr (which SIGKILLs after 100 ms),
`vscode-languageclient` sends `shutdown`, awaits the response, then sends
`exit`. A server that exited on `shutdown` without responding would produce a
five-second stall and then a forced kill on every window close. *Holds today;
the recorded transcript ends on the `shutdown` response followed by `exit`.*

## From Helix (ls06, `25.07.1`)

Helix is the first client that is *neither* hand-rolled nor well-behaved: it
implements the protocol competently and then declines to finish it.

**Survive a client that never sends `shutdown` or `exit`, leaving no orphan.**
Helix sends **neither**, and this was verified across all three quit paths —
`:q`, `:qa` and `:q!` — each producing a recorded session that ends at the last
response with no handshake at all. It is a different shape from facsimile's
(which SIGTERMs then SIGKILLs) and fackr's (which sends both and then kills):
helix simply drops the process. So the server must treat stdin EOF as a normal
end of session and exit cleanly on it. *Holds today; asserted by
`tests/semantics.rs::a_client_that_vanishes_leaves_no_orphan`, and confirmed by
`transcripts/helix/smoke.jsonl` ending on the `formatting` response with no
orphaned process left behind.*

**Answer `utf-8` when the client offers all three with utf-8 first.** Helix
declares `general.positionEncodings: ["utf-8", "utf-32", "utf-16"]` — note the
order differs from Neovim's `["utf-8", "utf-16", "utf-32"]` while producing the
same negotiated result, because wolf's own preference is what decides. Like
Neovim, helix converts correctly to whatever is negotiated, so a *wrong* choice
here is invisible rather than broken, which is worse. *Holds today; asserted by
`profiles/helix.json`'s `expects_encoding`.*

**Do not require `textDocument/didSave`.** The recorded session contains none;
diagnostics arrive on open and on change. *Holds today.*

**Expect `documentSymbol`, `hover` and `codeAction` only when asked.** Unlike VS
Code — nine unprompted `codeAction` requests in a 42-record session — helix
issues each exactly once, when the user presses the key. The 17-record helix
transcript contains no unrequested traffic of any kind. That is recorded not as
a constraint but as the *contrast*: the request-rate constraint VS Code imposes
is a VS Code property, not an LSP one, and a server tuned only against helix
would be surprised by it.

## From Emacs / eglot (ls06, `30.2`, eglot 1.17.30)

**Answer `utf-8` even though the client asks for `utf-32` first.** eglot
declares `general.positionEncodings: ["utf-32", "utf-8", "utf-16"]`, and wolf
answers `utf-8` because the *server's* preference decides. eglot is therefore
the first client tracked here whose own first choice the server declines — every
other client either offers one encoding, or offers utf-8 first. If the
negotiation rule ever changed to honour client order, this is the client where
it would show up first and silently: eglot converts correctly to whatever is
negotiated, so the columns would stay self-consistent and merely be different.
*Holds today; asserted by `profiles/emacs.json`'s `expects_encoding` and by the
negotiation recorded in `transcripts/emacs/smoke.jsonl`.*

**Expect `workspace/didChangeConfiguration` immediately after `initialized`,
carrying nothing useful.** eglot sends it on connect with the (empty) workspace
configuration, before any user action. A server that treated an unsolicited
`didChangeConfiguration` as an error, or that waited for settings before serving,
would break a client behaving perfectly. *Holds today — `wolf lsp` reads no
settings and ignores the notification.*

**`shutdown` then `exit`, and the client waits for the `shutdown` response.**
Same contract as `vscode-languageclient`. eglot additionally **reconnects** on an
unexpected server exit (`eglot-autoreconnect`, default 3 s), so a server that
exited on `shutdown` without responding would not merely stall — it would be
respawned, and the user would see an editor that appears to work while leaking a
process per window close. *Holds today; the recorded transcript ends on the
`shutdown` response followed by `exit`.*

**Tolerate two `didOpen`s before the first request.** eglot opens every already-
visiting buffer in the project the moment it connects, so a session that starts
with two files visited sends two `didOpen` notifications back to back and
expects a publish for each. *Holds today.*
