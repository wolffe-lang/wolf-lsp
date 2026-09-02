# Changelog

## le06 — 2026-09-02 — the generator tells the truth

The tmLanguage generator stops saying two things the spec does not, the
pin moves to wolf-lang **v0.2.3** (`3befc3e`), and a normalizer that had
been leaking absolute paths for eight sprints is caught by the pin bump
that re-recorded them.

**The generator, and the two blocks that were the acceptance.**
`raw-string` and `generalized-string` each carried `{ "include":
"#interpolation" }` while `[gram.lex.str.raw]` and `[gram.lex.str.gen]`
both say raw-mode — tree-sitter has always agreed, and the v0.2.3 EBNF
this sprint vendors now says it in productions too (`RAW_TEXT ::=
SCALAR*`, `GEN_TEXT ::= (SCALAR - ('"' | NL))*`; neither has an INTERP
alternative). Both rules carry an empty pattern list now. Measured
through wolf-book's own tmLanguage interpreter, before and after:

| block | before | after |
|---|---|---|
| ch02 §2.2 `r"C:\logs\{today}\wolf.log"` | `{`, `today`, `}` in interpolation scopes | one `string.quoted.other.raw.wolf` run, `13..37` |
| solutions `r"([{}])"` | `{`/`}` as hole punctuation | one raw run, `19..25` |
| solutions `r"when (a, b) { c[0] }"` | hole punctuation **and `0` as a numeric literal** | one raw run, `19..39` |

A fourth mover nobody had filed: `re"[a-z]+{2}"`'s regex quantifier
stopped reading as an interpolation hole. **wolf-lsp#4 closes.**

**The hole scans the literals (wolf-lsp#5), and the second half is a
closed match on purpose.** `#chars` drops straight in, so `'a'` inside
`"{(m + ('a' as int)) as char}"` paints as it does two lines above it.
The nested string ships as `#interpolated-string`, a closed match, and
**not** as `{ "include": "#strings" }` — that spelling is a reference
cycle (strings -> quoted-string -> interpolation -> strings), and the
one downstream consumer of this file that is not VS Code refuses it:
wolf-book renders every code block through `xtask/src/tm.rs`, which
expands includes eagerly and fails at LOAD with "include cycle through
`#strings` — the interpreter does not support recursion". Not a wrong
paint: no paint at all, book-wide. The closed form reads identically on
both engines, paints `"total"` in `print("{"total":<10}")`, and refuses
what it cannot prove — a nested literal carrying a brace or an escape
still goes unpainted, which is the same argument `#chars` has made since
le05. A generator test walks the emitted include graph and fails on any
cycle, so nobody re-introduces this one by accident.

**The pin, and two terminals the default was wrong about.** `vendor/
upstream/PIN` records `wolf 0.2.3 (wolfgang, pin 3befc3e)` and `lspconf
doctor` is **READY** here. The re-vendored data is one file:
`spec/grammar.ebnf` gains twelve lines that answer **wolf-lang#215** —
SCALAR, NL, STR_TEXT, the MULTILINE/RAW/GENERALIZED string productions
and CHAR_TEXT, the ones le05 filed as named-and-undefined. Those
productions are also the first to write `'"""'` and `'#'` as quoted
terminals, and xtask's partition sends every unlisted symbol to
`keyword.operator.wolf`. That default does not hold here: `#` in the
operator scope paints the fence of every `r#"…"#` and, worse, paints a
STRAY `#` — a stray byte by `[gram.lex.shebang]`, not language. Both
join `DELIMITERS` as the literal-form fragments they are, so the emitted
tmLanguage is unchanged by the bump.

**wolf-lang#199 gets its cleanest evidence yet.** le05 measured why the
pin clause's width belongs to the builder's clone; le06 shows two clones
on ONE BOX at ONE COMMIT disagreeing. The installed binary prints `pin
3befc3e` (seven); `git rev-parse --short` in this repo's `upstream/`
submodule (22,590 objects) answers `3befc3e8` (eight). So a developer
who follows `vendor/README.md`'s own pin-bump ritual gets a string this
file would have to record as eight digits, and one who installs the
published archive gets seven. PIN records what the ACQUIRED artifact
prints, and says so.

**Transcripts: 65 re-recorded, 60 header-only, and 5 that found a bug in
this repo.** Every scripted transcript re-recorded at the new pin; sixty
diffs are `@@ -1 +1 @@` with `wolf_pin` the only moved field. That is
what s134 predicted for the eighteen `annotate/*` files, recorded
against a branch binary with the pin unmoved — the caveat retires, and
it is the second consecutive branch-recorded set to re-pin without
moving.

The other five are the normalizer's. `Stage::Paths` is unconditional
because a machine-specific path in a committed artifact is the one thing
no assertion may depend on, and it walked string VALUES only. LSP has
exactly one map whose keys are data: `WorkspaceEdit.changes`,
`{ [uri: DocumentUri]: TextEdit[] }`. A rename or a code-action edit
answered to a client that does not declare `documentChanges` — nvim and
helix — stores its URIs in KEY position and nowhere else, so eight
records across six transcripts shipped a developer's home directory.
The sprint contract named this the "astral-path defect" and predicted
the fixtures root was missing from the elision list; measured, the
fixtures root was never the problem (`workspace fixtures` already
elides to `$WS`), the key-blindness was, and it reached
`vendor/upstream/samples` URIs just as readily. `elide_paths` also
learned the tilde form, because eglot names its workspace folder through
`abbreviate-file-name` and no absolute-prefix match can see `~/…`.

Six of the eight cleared on re-record. The two that did not are captured
sessions predating the fix, one of them recorded on a linux box, and a
script-less transcript cannot be re-recorded by design — they need a
re-CAPTURE, filed as **wolf-lsp#7**.
`tests/client_recorded.rs` now holds the property over every transcript
and every field, with those two in a waiver that is exhaustive in both
directions: a waived file that stops leaking fails the test too.

**The clients.** The vscode extension contributes `semanticTokenScopes`,
and the test that enforced their ABSENCE inverts to enforcing that the
mapping covers the server's closed legend exactly, in both directions.
This is not decoration: VS Code's fallback for a token type a theme has
no rule for is *no rule*, so serving semantic tokens without a mapping
would leave a file **less** coloured than the TextMate grammar left it.
Inlay hints needed nothing — there is no contribution point, whether
hints show is the user's `editor.inlayHints.enabled`, and nvim's and
helix's stay off by those editors' own defaults. The README sentence
"semantic tokens and inlay hints are absent, and a test enforces it"
retires. compat rows earn **0.2.3** on measured evidence; Zed keeps its
NO-SESSION caveat with the wasm component re-built locally today.

**wolf-lsp#3 closes.** The acquire step asked for
`wolf-<shortsha>-linux-x86_64.tar.gz`, a name `xtask dist` has never
published at any tag this repo has pinned. Both fields come out of PIN
now. Two more things would have failed the moment a download succeeded:
the archive is a directory, so `--strip-components=1` is what makes
`.wolf-bin/wolf` the path `lsp_harness::locate` looks for (and puts the
runtime lib beside the binary), and the matrix is three OSes while the
pattern named linux. v0.2.3 publishes an asset for every runner in the
matrix — including the linux/aarch64 one le05 recorded as missing — so
the triple is derived per host. Whether the lane LIGHTS is CI's to
answer; docs/MATRIX.md is re-stamped from a CI result, never from an
edit, so its rows do not yet claim green.

**THE SERVER LANE LIT, AND THEN IT WENT RED — measured, not predicted.**
Pushing the acquire fix was the first time `server-lane` had ever
acquired anything (run `33694888387`): the download step and `lspconf
doctor` both passed on ubuntu AND macos, and the next step failed. Not
the server — the six captured editor smokes, at pin `70bdd35`, which
`lspconf replay` refuses. That refusal has been documented-red in
`release-check 3b` for four sprints; it had simply never reached CI,
because CI had never had a binary.

So `replay` learns the distinction this repository already draws
everywhere else. A SCRIPTED transcript at another pin means somebody
forgot `lspconf rerecord` — a real error, still exit 2. A SCRIPT-LESS
one cannot be re-recorded by design; it needs a person and an editor.
Those six are a named `SKIP:` now, printed with both pins and the remedy
on every run, and the sixty-five transcripts that ARE at the pin get
replayed instead of the whole lane aborting before the first one.
Nothing that was checked stops being checked: they were never compared
against anything, they were killing the run.

**A LANE THAT RUNS FINDS THINGS, AND IT FOUND ONE IMMEDIATELY.** With
the six captures skipped, `server-lane` went green on ubuntu — acquire,
`doctor`, replay, one-truth, the five server-gated suites and the seeded
fuzz, all of them for the first time in this repository's history — and
red on **macos**, at `semantics::a_cancelled_request_completes_promptly_
with_request_cancelled`: `Timeout { awaited: "a
textDocument/publishDiagnostics notification for …/hello.lu", seen:
["response id=1 …"] }`. The two sibling tests in the same file, at 800 ms
and 1500 ms of injected slowness, passed; ubuntu passed all three.

Nothing was wrong with the server, and the first fix was not enough,
which is the interesting part. `slow_session` sets
`WOLF_QUERY_TEST_SLOW_MS`, which adds that many milliseconds at every
query checkpoint, and then waited on `DEFAULT_TIMEOUT` — a 20 s budget
whose own comment says it exists to cover a COLD non-resident compiler.
Adding one `ms` to it moved the failure from 20 s to 30 s and no
further: `didOpen` -> the first `publishDiagnostics` is an analysis
PIPELINE, not one query, so the setup wait pays the knob an unknown
number of times. Setup now gets `DEFAULT_TIMEOUT + 6 * ms` and the
caller gets `DEFAULT_TIMEOUT + ms` back, because every assertion past
the setup is about one query and handing it the setup's headroom would
let a slow server look prompt. Six is headroom, not a measured count.

This is what a dark lane costs. The bug was reachable for as long as the
suite has existed, it is a plain arithmetic error in a deadline, and it
could not be seen from one 18-core laptop — it needs a 3-core host
running eleven of these in parallel.

**Gates.** `cargo xtask ci` is green on **all 16 checked steps** — the
first time this repository has had no red one; seven stay PENDING on a
human act, as before. `replay` is 65 ok / 6 named skips, `onetruth` 10
samples x 9 profiles with zero divergences, `doctor` READY,
`grammar-drift`, `config-check`, `compat-check`, `sync-pin`,
`nvim-check`, `emacs-check`, `independence` and `fixtures-check` green,
and the workspace suite passes. On the branch, CI is green on `test`,
`independence`, `nvim plugin`, `vscode extension`, `vscode package`,
`emacs mode`, `helix config` and `zed extension` across the three
tier-1 OSes.

**A rig note that changed a verdict.** `/opt/homebrew/bin/cargo`
precedes `~/.cargo/bin` on this box's PATH and is not a rustup shim, so
it ignores `rust-toolchain.toml`: gates run through it were 1.98.0
Homebrew, not the pinned 1.97.1, and a `collapsible_if` that is `-D
warnings` under the pin passed silently under it. The same PATH is why
`--target wasm32-wasip2` reported "can't find crate for core" — cargo
1.97.1 spawning rustc 1.98.0, whose wasm std is not installed. Run the
gates here as `env PATH="$HOME/.cargo/bin:$PATH" cargo …`.

## s134 transcripts — 2026-09-02 — the server annotates (pinned at le06)

**The server annotates — signature help, semantic tokens, inlay hints.**
Recorded against the wolf-lang `s134` branch binary with the pin unmoved:
eighteen new transcripts under `transcripts/annotate/` (one script per rung
per maintained client — fackr, facsimile, nvim, vscode, helix, emacs), the
forty-seven existing ones re-recorded with the `initialize` answer as their
only diff (three providers gained), `lifecycle/unknown-method` re-targeted
at what the server still refuses. `lspconf`'s script DSL learned `req
signatureHelp <file> <l:c>`, `req semanticTokens <file> [l:c-l:c]` (a
range means `/range`) and `req inlayHint <file> <l:c-l:c>`. The answers
differ per profile by the profile's own declarations: the doc rides as
markdown where `signatureHelp.signatureInformation.documentationFormat`
lists it and plain text where the client declares nothing (facsimile
issues the request without declaring it — answered on merit, s122's
posture). `lspconf bench` before/after the s134 binary on the same
machine: `diagnostics-after-edit` p95 110.6 → 110.8 ms (p50 107.7 →
106.6), hover p95 0.2 → 0.1, cold 4.1 → 4.4 — every class inside its
budget. **MATRIX** gains the three rows. Nothing in the clients moved:
the vscode extension still contributes no `semanticTokenScopes` and its
test still enforces that (a client packaging decision for le06, now that
the server serves them), nvim's and helix's inlay hints stay off by their
own defaults, and the extension's README sentence on the two absences
is le06's to retire at the re-pin.

## le05 — 2026-09-02 — the editors navigate

The pin moves to wolf-lang **v0.2.2** (`8cda3aa`), the transcripts move
with it, and three documents that described a client stop being true
about it.

**The pin, and wolf-lang#199 answered.** `vendor/upstream/PIN` records
`wolf 0.2.2 (wolfgang, pin 8cda3aa)`, measured from the binary installed
on nomad-1, and `lspconf doctor` is **GREEN** here — `READY —
/Users/…/.local/bin/wolf serves LSP at pin 8cda3aa`. The re-vendored data
is one file: `spec/grammar.ebnf` takes the v0.2.2 deltas, and all ten
samples are byte-identical across the two tags.

The clause is **seven** hex digits where le04's was eight, and le05
measured why rather than picking one. `git rev-parse --short` sizes its
AUTO abbreviation to the object count of the clone it runs in, so the
width is a property of the **builder's clone**, not of the commit: in a
2086-object clone `8cda3aa41…` abbreviates to `8cda3aa` and the binary
built there prints `pin 8cda3aa`; the same commit built from a full clone
of wolf-lang would print `pin 8cda3aa4`, and `doctor` would refuse it. The
PIN is doing its job — its job is to record what ONE binary prints and
refuse everything else, and loosening it to accept a prefix would accept a
stale binary, the single failure the mechanism exists to prevent. The
defect is upstream in D57's clause, it stays filed as **wolf-lang#199**,
and `docs/COMPAT.md` now states the consequence for anyone who builds
their own `wolf` instead of acquiring the published one.

**The transcripts are at the pin, and s133's caveat is retired.** All
**47** scripted transcripts re-recorded; every diff is `@@ -1 +1 @@` and
the only field that moved is `wolf_pin`. The eighteen `navigation/*` files
are the ones that mattered: le04 recorded them against the wolf-lang
`s133` **branch** binary with the pin unmoved and predicted a header-only
diff at the re-pin. Measured — the `initialize` answer still carries
`definitionProvider`, `referencesProvider` and `renameProvider:
{prepareProvider: true}`, byte-identical. Every navigation row in
`docs/MATRIX.md` is now pinned by a transcript taken from a **tagged
release build**. `replay` 47/47, `onetruth` 10 samples × 9 profiles with
zero divergences, `verify` green. The six **captured** editor smokes are
NOT re-captured and still refuse replay at their own pin (`70bdd35`;
`67c977f` for fackr) — the standing posture, and the whole of what keeps
release-check 3b red.

**`cap` is a contextual keyword.** v0.2.2's `region_cap ::= 'cap' ':'
expr` (s132, `[mem.region.cap.1]`) put a new word terminal in the pinned
grammar, and `grammar-drift` refused to generate until it was classified
— which is exactly what that gate is for. It joins `rc` and `pool` in
xtask's `CONTEXTUAL` list: contextual, not coloured, because `cap` is an
ordinary local in any code that measures one.

**A stale test, red before le05 touched it.** `tests/negotiation.rs`
asserted that `textDocument/rename` answers `-32601`. s133 implemented
rename and did not update the list, so the test failed against any binary
that serves the capability `docs/MATRIX.md` credits it with — including at
trunk's own pin, measured. `signatureHelp` takes its place as the
real-but-unimplemented shape, and the positive half is now asserted too:
definition, references and rename must NOT answer MethodNotFound, so
moving a method off that list can never quietly mean "we stopped testing
it".

**The acquire lane's darkness has a new owner: us.** Measured 2026-09-02,
`gh release list --repo wolffe-lang/wolf-lang` reports **v0.2.2 as
Latest, not Draft**, carrying three tier-1 archives, and an
unauthenticated request for the download URL answers `200`.
**wolf-lang#200 is resolved** and `release-check` step 0 — a step whose
reason had been rewritten twice as the world moved — is a PASS for the
first time. What still keeps `server-lane` dark is a **stale glob in this
repo**: `.github/workflows/ci.yml` asks for
`wolf-<shortsha>-linux-x86_64.tar.gz` while `xtask dist` publishes
`wolf-<version>-<target-triple>.tar.gz` — filed as **wolf-lsp#3**, with the
two things a fix has to get right. le05 records it rather than rewriting a
CI lane it cannot run, and the matrix rows say "dark: acquire
glob stale" instead of "no pin-matched artifact", which had stopped being
true. (One real upstream gap remains: no linux/aarch64 archive at this
tag; wolf-lang trunk `4d9683d` repairs it for the next.)

**The facsimile mirror stops describing an editor that no longer exists.**
Re-read against facsimile trunk `a121ab3` (v0.35.0), whose PR **#5**
(merge `2f5d5f4`) closed FortranGoingOnForty/facsimile#4 — the issue le04
filed. Every sentence below was true when written and is not now:

- *"the client's static capability table still gates the keys off"* →
  routing reads the server's own advertised capabilities. `lsp_server_t`
  carries nine `supports_*` fields filled from the `initialize` reply, and
  `server_serves()` consults them; the static table is demoted in its own
  comment to "the floor used before that reply arrives". All three
  navigation rows are **reachable** in that editor now, not merely
  answerable by the server.
- *"its definition parser reads `Location[]` while it declares
  `linkSupport: true`"* → `definition_target()` parses `LocationLink`,
  preferring `targetSelectionRange`. Of the two one-line fixes offered in
  `docs/SERVER-CONSTRAINTS.md`, facsimile took the one that makes its own
  declaration true.
- *"completion must be a `CompletionList` — a bare array yields zero
  items"* → the popup falls through to a bare `CompletionItem[]`, with the
  wolf-shaped failure named in its comment.
- *"the capability flags drive facsimile's request routing"* and
  *"`CAP_DIAGNOSTICS` is read by no routing code anywhere"* → both
  retired; nine capabilities have call sites and `server_serves`
  special-cases diagnostics by name.
- *"three real parts of wolf lexical structure are out of reach"* → two.
  **`TOKEN_INTERP`** is a tenth token class with a theme role of its own
  (`syntax.interp`), driven by a new `interpolated_strings` language flag
  that `load_wolf_syntax` sets because every wolf string is an f-string.
  `{{`/`}}` are recognised rather than invisible. What is still out of
  reach is the expression *inside* a hole.
- *"in worktree, uncommitted"* (every row of `patches/STATUS.md`) →
  **upstreamed**, as one squashed commit `21e58aa`, which means the
  one-PR-per-group decomposition never happened and there is nothing in
  facsimile's history to point the labels at. Flagged there, with the
  number collision it creates: that table's own "PR5" is an unwritten
  offer, unrelated to facsimile's real PR #5.
- *"facsimile's Python table lists `\"` before the triple form and has
  exactly that bug today"* → fixed upstream at `d9fafb3`, whose sibling
  `c6f8878` is `docs/UPSTREAM.md`'s PR6. **That row moves to `MERGED`**,
  re-verified against trunk.
- A new section records what the mirror never had: **`note_lsp_error`**,
  the one-message channel that finally makes an LSP failure visible. Keys
  used to fail silently — the messages existed but went through
  `set_status_message`, which does not set `g_lsp_ui_changed`, so the main
  loop never redrew. `note_unserved_capability` now tells a user "wolf lsp
  does not serve go-to-definition" instead of letting the key do nothing,
  which is one more reason `wolf lsp` must keep answering `-32601` by name.

**Gate posture.** `doctor` GREEN, `verify` green, `replay` 47/47 scripted,
`onetruth` zero divergences, `config-check`, `grammar-drift`,
`compat-check`, `nvim-check`, `independence`, `vendor-check`,
`fixtures-check`, `sync-pin` all green, and the Rust suite passes.
`cargo xtask ci` is red on exactly one step, `release-check 3b` — the six
captured editor smokes, which cannot be cleared without driving six real
editors and which `docs/MATRIX.md` has named as owed since le01. Measured
on this box: **trunk was red on five release-check steps (2a, 2b, 3b, 5a,
5b); le05 leaves one.** The range in every `compat.json` moves to a
0.2.2 pin range — one version wide, as `plugin_spec.lua` asserts and the
pre-1.0 posture requires.

## s133-transcripts — 2026-09-02 — the server navigates

wolf-lang's s133 branch serves `textDocument/definition`,
`textDocument/references` and `textDocument/rename` (+`prepareRename`)
— wolf-lang#208's three dead keys. This branch is the harness half,
**recorded against that branch's binary with the pin UNMOVED**:
`vendor/upstream/PIN` still names `v0.2.1`, every transcript header
still says so, and the sessions were driven with `WOLF_BIN` pointed at
a `wolf 0.2.1+dev` build of `s133` (the Doctor's version check was
satisfied by a local, uncommitted edit of the PIN's `version` line for
the duration of the recording — the honest statement of what these
bytes were earned against). le05 re-pins at the tag that carries s133
and re-records; that is expected to be a header-only diff.

**21 new scripted sessions.** `transcripts/navigation/` holds one script
per rung per maintained client profile — `definition-`, `references-`,
`rename-` × fackr, facsimile, nvim, vscode, helix, emacs — over
`resolve/two_mod` (a cross-file item, a module name) and `hello.lu` (a
local, a prelude name). The request lines are identical across the six;
the answers are not, and that is the point: `LocationLink[]` to the five
profiles that declare `linkSupport`, `Location[]` to helix;
`documentChanges` to the five that declare
`workspaceEdit.documentChanges`, the `changes` map to nvim. Rename's
refusal set rides every one of them as `-32803` errors, and
`docs/COMPAT.md` states it in a table. `transcripts/encoding/` gains
`astral-navigate-{utf8,utf16,utf32}` — the astral-utf16 shape applied to
navigation: the same `bmp` requested and answered at three different
columns.

**The capability snapshot regains three providers.** All 26 existing
sessions re-recorded: every diff is the header date plus
`definitionProvider: true`, `referencesProvider: true`,
`renameProvider: {prepareProvider: true}` in the `initialize` answer —
byte-identical otherwise, which is the regression claim for everything
s122 and before served. `lifecycle/unknown-method` probed
`textDocument/definition` as its unimplemented method and now probes
`signatureHelp` (s134's), since the old probe is answered.

**Two harness changes.** `lspconf`'s script DSL learned `definition`,
`references … decl|nodecl`, `prepareRename` and `rename … <newName>`
(the `raw` escape hatch would have done, but a verb per rung keeps the
scripts reviewable). The `set:` matcher accepts `null` — the array-
valued methods may all answer "nothing here", and a null is not a set of
anything; it matches only itself.

**MATRIX** gains a capability-rows table (what the server serves and
which transcript pins it per client); **SERVER-CONSTRAINTS** records
where facsimile's two response-shape traps landed: the server follows
the protocol (bare completion array; `LocationLink[]` to a `linkSupport`
declaration — facsimile's own), and the one-line fix is facsimile's,
filed on FortranGoingOnForty/facsimile#4 together with the static
capability table that still gates F12/Shift+F12/F2 off. The `lspconf
bench` table before/after the s133 binary: `diagnostics-after-edit` p95
105.7 → 105.7 ms (p50 104.9 → 104.6) — the number near perception holds.

## le04 — 2026-09-01 — the pin and the escape

The upstream pin advances to **`75fd2d0`, the `v0.2.1` release tag** —
the first pin this repo has ever placed *at* a tag rather than on an
off-tag trunk commit, which changes two things and re-lights one gate's
reason.

**The version string loses its `+dev`.** D57 grants the release stamp
only when the builder sets `WOLF_RELEASE=v{version}` *and* that exact
tag points at HEAD, so this pin records the bare
`wolf 0.2.1 (wolfgang, pin 75fd2d0b)`. `doctor` says READY on it and
`compat`'s `version_number` — which learned semver build metadata at
le03 — reads `0.2.1` off it unchanged, with the pin clause correctly
not mistaken for a version (both now asserted).

Two things about that string were **measured, not assumed**, and both
contradicted the obvious guess. The pin clause is *eight* hex digits,
not seven: wolf-lang's own stamp runs `git rev-parse --short` with
git's auto width, which has grown past seven in a full clone of that
repo. And this repo's README recipe — a hand-rolled `cargo build` with
`WOLF_COMMIT=$(… --short=7 …)` in front of it — could not have produced
the pinned string at all, on two counts: it never grants the release
half, and it disagrees on the width. The recipe now runs upstream's own
`cargo xtask dist`, so the stamp is upstream's answer instead of our
restatement of it.

**The ritual, in full.** Re-vendored (samples byte-identical,
`grammar.ebnf` moves). All 26 scripted sessions re-recorded; replay and
onetruth green over 10 samples × 9 profiles, zero divergences. Every
re-recording is a one-line diff — only the header's date and `wolf_pin`
move — so the server's wire behaviour is byte-identical across
`83f83bb..v0.2.1`, which is what a patch release ought to look like and
is worth having checked rather than assumed. Compat rows earn `0.2.1`,
COMPAT regenerated, MATRIX re-stamped.

**The escape reaches the editors.** tree-sitter-wolf re-pins at its le04
head `bba5274`: v0.2.1's `UNI_ESC` bounds `\u{…}` at one to six hex
digits (wolf-lang#189, closed at the prose's letter), and the grammar
takes the bound where it can. Zed's shipped `highlights.scm` needed no
re-sync — the queries are byte-identical at this bump — and the captures
were spot-verified with the shipped queries: `'\u{4}'`, `'\u{000041}'`
and `'\u{1F43A}'` still `@constant.character`, the in-string escape
still `@constant.character.escape`, and the seven-digit `'\u{0000041}'`
goes **unpainted**, which is the refusal a tree-sitter grammar can
express. The VS Code TextMate char rule takes the same bound: it had
written `HEX_DIGIT+` straight from the old `CHAR_ESC` while
`syntax/wolf.vim` already wrote `\x\{1,6}` — the two editors disagreed,
and the amendment says vim was right.

Three pieces of prose were **wrong, not merely stale**, and are fixed
rather than re-stamped:

- `wolf-mode.el`'s syntax table said "No character literal in wolf" —
  false since s121/D58, and this pin's whole subject is the char escape.
  The behaviour is kept (a `'` that opened a string would swallow every
  apostrophe in every comment); the reason is corrected, and the
  unpainted literal is named as owed rather than denied.
- `UPSTREAM.md`'s tree-sitter-wolf row described an empty seed repo and
  commented-out grammar blocks — three sprints after le02 wrote that
  grammar and made the blocks live. Flipped to `MERGED` with the true
  note. That file's own preamble is about exactly this failure, so the
  "what refreshes this file" paragraph now names both directions it has
  been caught in.
- `release-check` step 0 said "no tagged wolf-lang release exists",
  false since v0.1.0. Its honest blocker today is below.

**Standing obligations, re-checked and re-stated — none absorbed:**

1. **Zed and JetBrains have still never been run.** Zed's wasm build was
   re-run locally at this pin (its compat row keeps the caveat in full);
   `profiles/zed.json` and `transcripts/zed/smoke.jsonl` are still owed,
   and JetBrains stays `NEVER` by design (T3).
2. **The six captured editor smokes keep their recorded pins**
   (`70bdd35`; `67c977f` for fackr) and refuse replay at `75fd2d0` —
   release-check 3b is red on exactly this until someone drives each
   real editor again. Re-capturing them was not this sprint's act.
3. **The Windows stdio quarantine stands at `75fd2d0`**: re-read at the
   new pin, the three stdio publish tests in wolf-lang's
   `lsp_one_truth.rs` are still `#[cfg_attr(windows, ignore)]`
   (backlog `lsp-windows-stdio`), byte-identical to `83f83bb`.
4. **The fackr/facsimile patch series still carry their re-verification
   obligation** — unpinned upstreams, series written against one commit.
   Their inventory ledgers were re-read, not re-stamped:
   `BUILTIN_TYPES` is byte-identical across `83f83bb..v0.2.1`, so the
   drift is still seventeen names, neither wider nor narrower.
5. **The astral-plane gap stays open**, and le04 measured it instead of
   repeating it: a sweep of the whole 467-file corpus finds zero ZWJ and
   zero combining marks, in raw bytes or escape-spelled — while the spec
   now *uses* a combining pair as a worked example (`'e\u{301}'` is two
   scalars). `fixtures/astral.lu` stays under its gap entry.
6. **The acquire step stays dark, for a NEW reason.** Every earlier pin
   was an off-tag sha no published asset could match. This one is a
   release tag and wolf-lang's v0.2.1 release *does* carry four tier-1
   assets — but the release is a **DRAFT**, as is v0.2.0; only v0.1.0 is
   published. A draft's assets sit behind an `untagged-…` URL and need
   an authenticated request, so a user still cannot acquire the binary
   these clients are verified against. Reported upstream, not worked
   around. The three-OS server-lane claim stays CI's to make, never a
   single host's (D35).

**Known gaps:** **D67** (ruled 2026-09-01) makes pattern separators
required — `','` separates fields and `'..'` follows a separator like
one more member — and wolf-lang's s131 lands the wolfgang tightening on
trunk **this wave**, after this pin. The editors deliberately do NOT
take it here; they take it at their next pin (le05-era). Until then
tree-sitter-wolf still parses `Point { x .. }`, so a lax-comma file that
highlights cleanly today will start **refusing under a future `wolfc`**,
and that is expected rather than a regression. (`Point { x y z }` and
`(a b)`, the other laxities D67 names, already fail to parse in the
pinned grammar, so exactly one spelling changes for the editors.) D67
names wolf-lang#190 as its tracker, but that issue is CLOSED
(`COMPLETED`, 2026-09-01, seconds after the v0.2.1 release draft) while
its own last comment says it stays open — flagged on the issue, not
reopened, since reopening is the orchestrator's call.

## le03 — 2026-08-31 — the grammar catches up

The upstream pin advances four days (s121–s129 drift) to `83f83bb`,
under the ritual: re-vendored (samples byte-identical, `grammar.ebnf`
moves), all 26 scripted sessions re-recorded, replay and onetruth
green over 10 samples × 9 profiles with zero divergences. The pinned
version string is now the `WOLF_COMMIT`-stamped build's (D57 — an
unstamped build prints `+dev.unknown`, which is the stale-binary hole
doctor exists to close), and the compat machinery reads through the
semver build metadata.

tree-sitter-wolf re-pins at its le03 head `09b3ca3` — char literals
(`[gram.lex.char]`, D58), D63 binder comma groups, struct patterns
(`[gram.pat.struct]`) — with Zed's shipped `highlights.scm` re-synced
and the captures verified over the new nodes: char literals as
`@constant.character`, struct-pattern heads as `@type`, fields as
`@variable.other.member`, group binders as locals definitions. The
type inventories catch up too: `char` joined `BUILTIN_TYPES` at s121,
so the vscode TextMate grammar (which also gains a closed char rule
and `#![…]` attributes), `syntax/wolf.vim` and `wolf-mode.el` all
paint it. Trunk CI was red since the le02 merge — le02 flipped the
grammar blocks live without flipping `config-check` — and the gate
now asserts the live posture instead: one grammar pin, two spellings,
held equal.

**Standing obligations, re-checked and re-stated — none absorbed:**

1. **Zed and JetBrains have still never been run.** Zed's wasm build
   was re-run locally at this pin (its compat row keeps the caveat in
   full); `profiles/zed.json` and `transcripts/zed/smoke.jsonl` are
   still owed, and JetBrains stays `NEVER` by design (T3).
2. **The six captured editor smokes keep their recorded pins**
   (`70bdd35`; `67c977f` for fackr) and refuse replay at `83f83bb` —
   release-check 3b is red on exactly this until someone drives each
   real editor again. Re-capturing them was not this sprint's act.
3. **The Windows stdio quarantine stands at `83f83bb`**: the three
   stdio publish tests in wolf-lang's `lsp_one_truth.rs` are still
   `#[cfg_attr(windows, ignore)]` (backlog `lsp-windows-stdio`).
4. **The fackr/facsimile patch series still carry their
   re-verification obligation** — unpinned upstreams, series written
   against one commit — and their inventory ledgers' stale notes
   widened: the builtin type set is seventeen names now.
5. **The astral-plane gap stays open**: the `83f83bb` char-era
   witnesses add astral char literals, but no corpus sample carries
   combining marks or ZWJ, so `fixtures/astral.lu` stays under its
   gap entry.
6. **The acquire step stays dark** until wolf-lang publishes an
   artifact at this pin — v0.1.0 and v0.2.0 are ancestors of the pin,
   not at it — and the three-OS server-lane claim stays CI's to make,
   never a single host's (D35).

## le01 / le02 — 2026-08-28 — the editors go live at a real pin

The upstream pin advances seventeen days (s97–s120 drift) to f9ee9aa:
all 26 scripted sessions re-recorded, replay and onetruth green over
10 samples × 9 profiles with zero divergences. Helix and Zed
highlighting goes live — tree-sitter-wolf pinned at 0458cdf, Zed
ships its grammar key and highlights.scm — closing the track's
largest standing gap. The VS Code grammar snapshots re-recorded (the
extracted EBNF now carries the full precedence climb, so `==` and
`&&` are one token each). The upstream ledger was reconciled against
reality: six facsimile rows flipped NOT SUBMITTED → MERGED, and the
expired "wolf-lang tags no release" blanket retired — v0.1.0 shipped
2026-08-12, so the compat rows now earn a real 0.1.0 pin range.

## Quiet — 2026-08-12 → 2026-08-27

Two housekeeping commits only (GPL-3.0-or-later across server and
clients per D41 as amended; a prose pass correcting stale counts).
The standing obligations owed out of the track stood as written: Zed
and JetBrains capability profiles never run (stamped NEVER, not
dated), no tree-sitter grammar (until le02 above), and the Windows
stdio quarantine. Nothing was published anywhere — every channel is
built and rehearsed offline, gated on a human act.

## ls00–ls07 — 2026-08-09/10 — the whole editor layer, in one arc

The track's first arc, opened and closed in two days:

- **ls00** — workspace, three-OS CI, the wolf-as-binary pin strategy
  (the server is `wolf lsp` itself; this repo never builds the
  compiler in CI), and the transcript format.
- **ls01** — `lspconf`, the conformance harness: recorded JSON-RPC
  record/replay, capability profiles, position-encoding and
  cancellation suites, seeded partial-edit fuzz, latency budgets, and
  `onetruth` — LSP diagnostics == build diagnostics, per sample (D34
  made falsifiable; it found and closed DIV-LSP-001).
- **ls02/ls03** — the daily drivers: fackr (utf-32 profile, 19-record
  live smoke) and facsimile (utf-16 profile, 15-record smoke), each
  with integration docs, patch sets, and a server-constraints ledger.
- **ls04/ls05** — the mainstream pair: `wolf.nvim` (dual-purpose
  native/lspconfig entry, spec-derived syntax, checkhealth, vimdoc;
  20-record live smoke negotiating utf-8) and the VS Code extension
  (spec-derived TextMate grammar with a drift gate, 42-record real
  session under xvfb, vsix installs clean at 316KB).
- **ls06** — the config tier: Helix and Emacs (eglot) exercised with
  recorded sessions, Zed built for wasm32-wasip2, JetBrains
  documented with an honest NEVER stamp; `docs/MATRIX.md` with
  staleness stamps and a promotion policy.
- **ls07** — packaging and release: compat schema with earned ranges,
  `release-check`, distribution docs naming the human act each
  channel waits on, and the upstream patch ledger.

Nine matrix rows, six with a recorded session, two never run at all —
and the matrix says which is which.
