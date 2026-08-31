# Changelog

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
