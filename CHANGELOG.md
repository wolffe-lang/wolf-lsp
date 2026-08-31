# Changelog

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
