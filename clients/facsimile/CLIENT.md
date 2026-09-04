# What facsimile's LSP client actually is

Read from the source at `a121ab3` (trunk, v0.35.0 — re-read at le05 and at
**le07**, which found trunk unmoved: `a121ab3` is still the tip; first read at
`1242ffa`), not from `docs/LSP_GUIDE.md`, which documents gutter markers that
are commented out.

**A version-site drift worth reporting back, found at le07's re-read.** The
commit `a121ab3` is subject-lined "Bump to 0.35.0" and touches exactly two
files — `VERSION` and `src/version_module.f90` (`VERSION = '0.35.0'`). It does
NOT touch `fpm.toml`, which still declares `version = "0.34.0"`. So facsimile
has three version sites and a bump that moves two of them, and a package built
by `fpm` reports a different version than the editor prints. This mirror keeps
citing v0.35.0 because that is what the binary says, and the discrepancy is
reported upstream rather than papered over here. Every bullet here is either an ls01 transcript
case or a stated server constraint, and **none of them becomes a workaround
inside `wolf lsp`**.

**What moved at facsimile PR #5** (merge `2f5d5f4`, in trunk): request routing
now reads the server's own advertised capabilities instead of a static table,
`LocationLink` is parsed alongside `Location`, a bare `CompletionItem[]` is
parsed alongside `CompletionList`, and a status channel (`note_lsp_error`)
carries LSP messages to a status line that actually redraws. Three of this
file's claims died with it and are rewritten below rather than left standing.

facsimile is the strictest constraint source this project has on what a server
may *require* of a client. Its client is deliberately minimal, and the
minimalism is load-bearing: it found the one constraint (§ "Server→client
requests") that would hang an editor forever.

## Declared capabilities — the complete list

`create_initialize_request`, `src/lsp/lsp_protocol_module.f90`:

```
textDocument.completion      { dynamicRegistration: false, contextSupport: true }
textDocument.hover           { dynamicRegistration: false }
textDocument.definition      { dynamicRegistration: false, linkSupport: true }
textDocument.references      { dynamicRegistration: false }
textDocument.documentSymbol  { dynamicRegistration: false }
textDocument.formatting      { dynamicRegistration: false }
workspace.applyEdit          true
workspace.workspaceEdit      { documentChanges: true }
general.positionEncodings    ["utf-16"]      <- added by this sprint
```

That is all of it. There is no `synchronization` object, no
`publishDiagnostics`, no `codeAction`, no `rename`, no `signatureHelp`, no
`workspace.symbol`, no `workspaceFolders`.

**Not declared but used anyway.** `codeAction` (F10), `signatureHelp`,
`rename` (F2) and `workspace/symbol` (F6) requests are all issued without the
corresponding client capability, and diagnostics are handled without declaring
`publishDiagnostics`. A server that took the capability document literally and
refused to answer would be within its rights and would break this editor. wolf
answers on merit rather than on declaration, which is why it works.

## Position encoding — correct, and now also declared

facsimile is the mirror image of fackr. Its columns are **UTF-16 code units**,
converted at every outbound and inbound site through `utf8_char_col_to_utf16` /
`utf16_to_utf8_char_col` (`src/utils/utf8_module.f90`), and the conversion is
correct including non-BMP. This is the reference implementation ls02 borrowed
from.

Before this sprint it declared nothing, so it was correct only because UTF-16
is the protocol's default — "right because nobody wrote it down". It now
declares `general.positionEncodings: ["utf-16"]`, and the negotiation is
visible on the wire in `transcripts/facsimile/smoke.jsonl`:

```
c2s  initialize   general.positionEncodings = ["utf-16"]
s2c  response     capabilities.positionEncoding = "utf-16"
```

**UTF-16 alone, deliberately.** wolf's preference order is utf-8 → utf-16 →
utf-32, so offering utf-8 as a "fallback" would get utf-8 and silently corrupt
every column past a multi-byte character. Report 09's instinct to offer a list
is wrong against this server, in the opposite direction from fackr's.

## Sync

Full text only. `didChange` always carries one `contentChanges[0].text` with
the whole buffer, debounced 0.5 s (`document_sync_module`). No incremental
path, and the server's `textDocumentSync.change` is never read — wolf
advertises `1` (Full), so they agree by luck rather than negotiation.

`didChange` always carries `"version": 1` — the version field is hardcoded, so
it conveys no ordering information at all.

## How the client tells its user something went wrong (PR #5)

New at `a121ab3`, and worth recording because le04's mirror had nothing on it:
a key that reached no server used to fail **silently**. The messages existed —
they were written with `set_status_message` — but that call does not set
`g_lsp_ui_changed`, and the main loop only redraws when it collects one, so a
keypress that changed nothing else never repainted and the message was never
seen. `command_handler_module.f90:384-387` says so in place.

The fix is a one-message channel out of the LSP layer, which is compiled
before the renderer and so cannot touch the status line itself:

- `note_lsp_error(msg)` (`lsp_server_manager_module.f90:601-607`) remembers one
  message. First one wins — `if (g_lsp_error_pending) return ! first one wins;
  no spew`.
- `take_lsp_error(msg)` (`:610-620`) drains it.
- `app/main.f90:699-705` drains it inside the main loop, calls
  `set_status_message`, and sets `g_lsp_ui_changed` so the frame actually
  repaints.

`note_unserved_capability` (`:1208-1236`) is the producer that matters to this
project: when routing declines a key because the server did not advertise the
capability, the user now reads e.g. `wolf lsp does not serve go-to-definition`
instead of watching a key do nothing. **This is a constraint relaxed, not
added** — an unserved method is now a visible refusal in this editor, which is
one more reason `wolf lsp` must keep answering `-32601` by name rather than
staying silent.

## Server→client requests — the finding this client exists to produce

`handle_request` (`src/lsp/lsp_server_manager_module.f90:853` — it was `:847`
before PR #5 moved it) is **an empty stub** — its entire body is a `TODO` and an `if (.false.) print *, …` that
exists only to silence the unused-argument warnings, which is as explicit as a
comment can be about the fact that nothing is answered.
`workspace/configuration`, `client/registerCapability` and
`window/workDoneProgress/create` receive **no reply of any kind, ever**.
`window/showMessage` and `window/logMessage` are received and discarded.

A server that blocks waiting on any of those hangs this editor forever, with no
error, no timeout and no log line. This is the single most valuable thing the
tier-0 integration produced, and it is a constraint on every future wolf
capability, not a one-time check — see
[`docs/SERVER-CONSTRAINTS.md`](../../docs/SERVER-CONSTRAINTS.md).

Verified against the pinned server: `wolf lsp` constructs only
`Message::Response` and `Message::Notification`, never `Message::Request`
(`crates/wolf_lsp/src/server.rs`, whose inbound `Message::Response(_)` arm is
commented "we send no server→client requests"). The recorded session confirms
it empirically — 15 records, zero server→client requests, session completes
normally.

## Cancellation

No `$/cancelRequest`, ever. In-flight requests are tracked in a fixed
`pending_requests(100)` table and simply abandoned. A response to a request the
user has moved past is parsed and dropped.

## JSON limits

`src/lsp/json_module.f90` is hand-rolled, and three of its properties are
server constraints:

- **`\uXXXX` is not decoded, in either direction.** An escaped sequence arrives
  at the user as the literal six characters. The server must emit raw UTF-8 —
  pinned by `tests/encoding.rs::the_server_emits_raw_utf8_and_never_backslash_u_escapes`,
  which reads raw frame bytes rather than parsed JSON, because parsing is
  exactly what would hide the bug.
- **All numbers are `real64`**, so request ids round-trip through a double
  (bounded by 2^53) and **string ids are unsupported**.
- **Parse and serialize are O(n²)** in message size, and `read_buffer` regrows
  by copy. A large completion list or a long markdown hover visibly stalls the
  editor. Keep responses small.

The recursive descent parser has no depth limit, and `Content-Length` is
located with an unanchored `index()` scan rather than an anchored one.

## Shutdown

There is none. facsimile sends neither `shutdown` nor `exit`: it SIGTERMs the
server and SIGKILLs it 100 ms later. The recorded transcript therefore ends at
the last response, and the server must survive having its pipes closed under it
without leaving an orphan.

This is also why the pin's new "bare `exit` exits 1" behaviour is invisible
here — facsimile never sends `exit` at all.

## What is wired, and what only looks wired

| feature | key | state |
|---------|-----|-------|
| diagnostics | — | push, on open and on every debounced change |
| document symbols | `F4` / `Alt+O` | works; panel renders |
| formatting | `Alt+Shift+F` | works — **after this sprint's fix**; the binding was dead (see below) |
| hover | `Ctrl+H` | request goes out, response arrives, tooltip renders |
| code actions | `F10` / `Alt+.` | wired; wolf serves fully-resolved quickfixes |
| completion | `Ctrl+Space` | wired and **reachable**; wolf has served completion since s122, and `completion_popup_module.f90:102-108` now reads a bare `CompletionItem[]` as well as a `CompletionList`, which is the shape wolf answers |
| definition / references / rename | `F12` / `Shift+F12` / `F2` | wired, **served since wolf-lang s133** (`transcripts/navigation/*-facsimile.jsonl`), and **reachable since facsimile PR #5**: routing consults `supports_definition`/`supports_references`/`supports_rename` (filled from the `initialize` reply, read by `server_serves`), and `definition_target` parses the `LocationLink` shape the client's own `linkSupport: true` asked for |
| workspace symbols | `F6` | wired; wolf does not serve it |

**Known gaps, re-checked at `a121ab3`.** Absent entirely: semantic tokens (the
string does not occur in the repo), pull diagnostics, `didClose`
(`notify_file_closed` is public and imported in two places but has zero call
sites, so documents accumulate for the session), gutter markers for
diagnostics (the code is commented out, though `docs/LSP_GUIDE.md` claims
otherwise), `$/cancelRequest` (absent from `src/`), `shutdown`/`exit` (neither
string appears in `src/lsp/`), `\uXXXX` decoding in `json_module.f90`, and
server→client request handling (`handle_request` is still the empty stub —
re-read at le07 and still a `TODO: Handle server requests` with a `.false.`
guard to silence the unused arguments, at `lsp_server_manager_module.f90:853`).
`didChange` still sends `"version": 1` on every notification — see the
re-recorded bullet in `README.md`: an optional `version` argument and a live
counter both exist now, and neither of the two call sites connects them. None of these moved at PR #5; the
things that DID move are the three struck rows above.

**The two response-shape traps are closed.** They were: completion had to be a
`CompletionList` (`{"items": […]}`) because a bare array yielded zero items,
and definitions had to be `Location`/`Location[]` because `LocationLink[]` was
not parsed despite `linkSupport: true` being advertised. PR #5 took the second
option on both — the parser now accepts what the client declares.
`completion_popup_module.f90:102-108` falls through to `JSON_ARRAY`;
`lsp_protocol_module.f90:665-697`'s `definition_target` reads `targetUri` and
prefers `targetSelectionRange` over `targetRange`, falling back to the plain
`Location` path. `test/test_lsp_response_shapes.f90` pins both, including that
the cursor follows `targetSelectionRange`. A server may now answer either
shape; wolf keeps answering the one the client's declarations ask for, because
that is what the transcripts pin.

## Bugs found in facsimile by integrating wolf

Both are wrong for every server, not just wolf, which is what makes them
patchable upstream (`CONTRIBUTING-patch.md`).

**Document formatting was unreachable from the keyboard.** The handler matched
`case('shift-alt-f')`, but nothing in `src/terminal/` ever produces that
string: `modifier_prefix` and `decode_csi_u` both emit `alt-` before `shift-`,
and every other chord in the file is spelled that way (`alt-shift-left`,
`alt-shift-j`, …). Formatting could not be invoked at all. Fixed by accepting
both spellings.

**The installer catalogue could not grow without an out-of-bounds write.**
`get_known_servers_count()` hardcoded `20` and `init_known_servers` filled
exactly 20 slots through an assumed-shape dummy that never consults `size()`.
Adding a 21st entry without bumping the count writes past the end of the array.
Fixed for this entry; the structural fix is offered separately.

**`notify_file_changed` wrote `/tmp/fac_didchange_debug.log` on every change.**
Before the guards, so it also fired for invalid and uninitialised servers — an
open/write/close on the hottest path in the module, and a unixism in a codebase
with a `check-windows` gate. Removed.
