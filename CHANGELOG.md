# Changelog

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
head `f704738`: v0.2.1's `UNI_ESC` bounds `\u{…}` at one to six hex
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
and that is expected rather than a regression. wolf-lang#190 stays open
as D67's tracker. (`Point { x y z }` and `(a b)`, the other laxities
D67 names, already fail to parse in the pinned grammar.)

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
