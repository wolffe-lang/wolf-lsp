# emacs

**Tier 2 — the config tier, and a delta from the sprint plan.** ls06 §3 places
Emacs in the *documented* tier, verified by a human at release time. It is
filed here at T2 instead because the sprint's own promotion rule —
"an editor reaches T2 when its config is machine-checkable in CI" — is met:
`clients/emacs/tests/mode-test.el` is nine ERT cases that need **no `wolf`
binary and no server**, so the lane is green on all three tier-1 runners. The
promotion is recorded in [`docs/MATRIX.md`](../../docs/MATRIX.md) with its
evidence, and the delta belongs in the campaign closeout.

There is a second, larger piece of evidence and it is deliberately *not* used
to claim T1: a real eglot session was recorded
([`transcripts/emacs/smoke.jsonl`](../../transcripts/emacs/smoke.jsonl)) and a
capability profile derived from it
([`profiles/emacs.json`](../../profiles/emacs.json)). That satisfies the
"a real client session can be recorded" half of the T1 criterion and not the
"replayed headlessly in CI" half, for the same reason no T1 row's server half
runs in CI today — there is no `wolf` release artifact to acquire.

- Upstream: `emacs-mirror/emacs`, read at `30.2` (Arch Linux `extra/emacs-nox
  30.2-3`), with **eglot 1.17.30 bundled** — `eglot.el` has shipped in-tree since
  Emacs 29, so there is no package to install for the LSP half either
- Capability profile: [`profiles/emacs.json`](../../profiles/emacs.json)
- Recorded session: [`transcripts/emacs/smoke.jsonl`](../../transcripts/emacs/smoke.jsonl)

## What is in here

```
wolf-mode.el              the whole client — a mode, and one eglot line
tests/mode-test.el        9 ERT cases, no wolf binary, no server
tests/server-test.el      1 ERT case: one session, eight `ert-info` sections; skips loudly at 77
```

No `.elpa` directory, no `Makefile`, no `Cask`, no recipe. **This is not a
package**, it is not on MELPA, and packaging it is not v1's problem (ls06 §3,
and ls07 owns distribution if it ever becomes one).

## Setup

`wolf lsp` **is** the compiler (D34), so there is no server to install and no
version to keep in sync with anything.

1. Put `wolf` on `PATH`.
2. Paste the snippet below into your init file.
3. Open a `.lu` file and `M-x eglot`.

That is the whole thing. There is no `capabilities` table to thread through, no
`:initializationOptions`, and no `wolf-mode-hook` you are expected to add
`eglot-ensure` to by hand — though `(add-hook 'wolf-mode-hook #'eglot-ensure)`
is the one line that makes the server start without `M-x eglot`, and it is
yours to add rather than ours to impose.

### The snippet, which is `wolf-mode.el` byte for byte

Not an illustration of it: `clients/emacs/wolf-mode.el` **is** these bytes, and
`wolf-the-readme-snippet-is-this-file-verbatim` in `tests/mode-test.el` fails
if they ever differ. Copy it into `~/.emacs.d/wolf-mode.el` and
`(require 'wolf-mode)`, or paste it straight into `init.el`.

```elisp
;;; wolf-mode.el --- Major mode for the wolf language -*- lexical-binding: t; -*-

;; SPDX-License-Identifier: GPL-3.0-or-later

;;; Commentary:

;; **Tier 2 — the config tier.**  This file is not a package and is not on
;; MELPA; it is the copy-pasteable snippet from `README.md', shipped as a file
;; so that it can be *run* by `emacs --batch' instead of only read.  A test
;; asserts that `README.md' contains these bytes, so the snippet a reader pastes
;; and the snippet CI executes cannot drift apart.
;;
;; Nothing here is a language server.  `wolf lsp' is the compiler (D34) and
;; serves diagnostics, hover, document symbols, formatting and code actions on
;; its own; this file supplies only what a server cannot: a major mode to hang
;; them off, the comment syntax, and the two numbers the formatter fixes.
;;
;; Every value is READ OFF the pinned toolchain rather than chosen:
;;   upstream/crates/wolf_fmt/src/doc.rs — INDENT = 4, WIDTH = 100
;;   vendor/upstream/spec/grammar.ebnf   — `reserved_kw', `//' line comments
;; `cargo xtask emacs-check' re-derives the keyword list below from the pinned
;; grammar on every CI run and fails on any difference.

;;; Code:

(defconst wolf-mode-keywords
  ;; reserved-kw-begin
  '("as" "asm" "assume" "borrow" "break" "comptime" "const" "continue" "copy"
    "defer" "distinct" "dyn" "else" "enum" "errdefer" "export" "extern" "false"
    "fn" "for" "freeze" "handle" "if" "impl" "import" "in" "let" "loop" "match"
    "move" "mut" "proc" "pub" "region" "return" "scope" "select" "shared"
    "spawn" "struct" "take" "trait" "true" "type" "unsafe" "use" "var" "weak"
    "when" "while")
  ;; reserved-kw-end
  "The closed set of 50 reserved words, verbatim from `reserved_kw'.
Enforced against the pinned grammar by `cargo xtask emacs-check'.")

(defconst wolf-mode-builtin-types
  '("Self" "bool" "byte" "char" "f32" "f64" "i16" "i32" "i64" "i8" "int"
    "str" "u16" "u32" "u64" "u8" "uint" "wrapping")
  "Type names that are NOT reserved words, so they live outside the markers.
The closed builtin set at pin 83f83bb (wolf_sema BUILTIN_TYPES — `char'
joined at s121; spec/10
writes the fixed-width scalars normatively), plus `Self'.
`type' and `region' are also type-level but are reserved, and are coloured
as keywords above rather than duplicated here.")

(defvar wolf-mode-font-lock-keywords
  `((,(regexp-opt wolf-mode-keywords 'symbols) . font-lock-keyword-face)
    (,(regexp-opt wolf-mode-builtin-types 'symbols) . font-lock-type-face)
    ;; Doc comments are a *prefix* of the line-comment form, so they are matched
    ;; before font-lock's syntactic pass would paint the whole line as a comment.
    ("^[ \t]*\\(///\\|//!\\).*$" . font-lock-doc-face))
  "Minimal keyword highlighting.  Deliberately nothing else.
Anything richer would be a hand-written parser competing with the one in
the compiler, and the compiler is the authority (D22).")

(defvar wolf-mode-syntax-table
  (let ((table (make-syntax-table)))
    ;; `//' to end of line.  **Wolf has no block comment** — "nesting arguments
    ;; lose to simplicity + lexer speed" — so `/' is comment-start only as the
    ;; second character of a pair, and there is no `*' comment syntax at all.
    (modify-syntax-entry ?/ ". 12" table)
    (modify-syntax-entry ?\n ">" table)
    ;; `IDENT ::= (`_' XID_Continue+) | (XID_Start XID_Continue*)'.
    (modify-syntax-entry ?_ "_" table)
    ;; No character literal in wolf, so `'' is punctuation and never a string
    ;; delimiter — otherwise every apostrophe in a comment opens a string.
    (modify-syntax-entry ?\' "." table)
    (modify-syntax-entry ?\" "\"" table)
    table)
  "Syntax table for `wolf-mode'.")

;;;###autoload
(define-derived-mode wolf-mode prog-mode "Wolf"
  "Major mode for editing wolf source (`.lu')."
  :syntax-table wolf-mode-syntax-table
  (setq-local font-lock-defaults '(wolf-mode-font-lock-keywords))
  (setq-local comment-start "// ")
  (setq-local comment-end "")
  (setq-local comment-start-skip "//+[ \t]*")
  ;; `wolf_fmt::doc::INDENT' = 4, and spaces: every byte of canonical output is
  ;; a space, so a tab in a wolf file is a formatting diff waiting to happen.
  (setq-local tab-width 4)
  (setq-local indent-tabs-mode nil)
  ;; `wolf_fmt::doc::WIDTH' = 100, so `fill-paragraph' and `display-fill-column-
  ;; indicator-mode' agree with the formatter instead of arguing with it.
  (setq-local fill-column 100))

;; `.lu' — lupus — and ONLY `.lu'.  `.wolfi' is deliberately absent: `wolfi' v0
;; is a *binary* format (magic bytes `WOLFI'), `wolf lsp' discovers modules by
;; `.lu' alone, and any mode deriving from `wolf-mode' would drag eglot along
;; with it — eglot matches parent modes.  A buffer that looks supported and is
;; not is worse than one with no mode at all.
;;;###autoload
(add-to-list 'auto-mode-alist '("\\.lu\\'" . wolf-mode))

;; The whole server integration, and there is no second half.  No `capabilities'
;; to thread through, no `on_attach', no contact-function branching: `wolf lsp'
;; is the compiler, so there is nothing to install and no version to keep in
;; sync with anything (D34).
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs '(wolf-mode . ("wolf" "lsp"))))

(provide 'wolf-mode)
;;; wolf-mode.el ends here
```

### lsp-mode users

`lsp-mode` is a different client with a different registry, so it needs its own
three lines. They are not tested here — no `lsp-mode` is installed on any
machine this repository runs on, and asserting against a package we do not have
would be the fiction `profiles/README.md` exists to forbid:

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(wolf-mode . "wolf"))
  (lsp-register-client
   (make-lsp-client :new-connection (lsp-stdio-connection '("wolf" "lsp"))
                    :activation-fn (lsp-activate-on "wolf")
                    :server-id 'wolf)))
```

`wolf-mode.el` above is still required — `lsp-mode` keys off the major mode
exactly as eglot does.

## What works

Proven in [`transcripts/emacs/smoke.jsonl`](../../transcripts/emacs/smoke.jsonl),
a real recorded session, with every assertion in `tests/server-test.el` checked
as it was recorded:

| feature | how | notes |
|---------|-----|-------|
| diagnostics | flymake gutter, `M-x flymake-show-buffer-diagnostics` | push, on open and on change |
| hover | `M-x eldoc`, `eldoc-mode` | `who: str`, range exact (3 characters) |
| document symbols | `M-x imenu`, `M-x xref-find-apropos` | `main` |
| formatting | `M-x eglot-format-buffer` | canonical bytes round-trip to zero edits |
| code actions | `M-x eglot-code-actions` | wolf's fix-its arrive fully resolved |
| syntax highlighting | — | independent of the server; keywords only, see below |
| comment toggle | `M-;` | `//` only — wolf has no block comment form |

## Known limitations — stated honestly

None of these is worked around here (D22: the editor layer must not launder
what the compiler said).

**Highlighting is keywords, types and doc comments, and nothing else.** There
is no indent function, no `treesit` integration, no context-sensitive faces for
paths, attributes or f-string interpolation. Anything richer would be a
hand-written parser in Emacs Lisp competing with the one inside the compiler,
and the compiler is the authority. The keyword list is the closed set of 50
`reserved_kw` entries, re-derived from `vendor/upstream/spec/grammar.ebnf` by
`cargo xtask emacs-check` on every CI run — so an invented keyword fails the
build rather than teaching a language that does not exist.

**`treesit` is wired to nothing, deliberately.** Emacs 30 has
`treesit-language-source-alist` and would happily build a `wolf` grammar —
except that `wolffe-lang/tree-sitter-wolf` is a seed commit with no
`grammar.js` in it (`b1b2c17`). A `wolf-ts-mode` today would be a mode with no
parser, which is worse than no mode.

**`.wolfi` is not in `auto-mode-alist`, and a test enforces it.** `wolfi` v0 is
a *binary* format — magic bytes `WOLFI`,
`upstream/crates/wolf_sema/src/interface.rs` — and `wolf lsp` discovers modules
by `.lu` alone (D32). Any mode deriving from `wolf-mode` would drag eglot along
with it, because eglot matches parent modes; so the honest answer is no mapping
at all rather than a buffer that looks supported and is not. This is the same
ruling ls04 and ls05 reached, arrived at from a different constraint.

**Three things do not work under `emacs --batch`, all of them test-harness facts
and none of them user-facing.** They are listed because each one produced a test
that looked green, or flaky, for a reason with nothing to do with wolf:

1. **`eglot-ensure` never connects.** It defers onto `post-command-hook`, and
   `--batch` has no command loop — so the session silently never starts and the
   run is green over zero exercised assertions. `tests/server-test.el` calls
   `eglot` directly.
2. **`eglot` never flushes changes.** Pending edits go out from an idle timer
   (`eglot-send-changes-idle-time`, 0.5 s) and `--batch` never goes idle, so
   `eglot--signal-textDocument/didChange` is called by hand.
3. **`accept-process-output` alone does not dispatch notifications.**
   `jsonrpc--process-filter` does not handle a message inline; it schedules the
   dispatch with `run-at-time`/`timer-activate`. `accept-process-output` reads
   the bytes and does not reliably run those timers, so a run could receive every
   byte of a `publishDiagnostics` and never call `eglot-handle-notification`.
   Pumping with `accept-process-output` *and* `sit-for` fixes it; waiting on a
   condition with a wall-clock deadline rather than a fixed iteration count is
   the other half, since `accept-process-output` returns as soon as any process
   has output and "60 × 0.05 s" is sixty events, not three seconds.

None of the three applies to an interactive Emacs, where the command loop, the
idle timers and the top-level loop all exist.

**Only linux was exercised.** `emacs --batch` is portable and the lane is in
the three-OS matrix, but this line is here so nobody reads the CI matrix as a
claim that predates its first green run.

## Verification, and where it lives

Two lanes, split by what each needs, so a failure names its own cause:

- **Mode lane** (`tests/mode-test.el`, 9 cases): `auto-mode-alist`, the
  `prog-mode` derivation, comment syntax through `syntax-ppss`, the apostrophe
  rule, the formatter's two numbers, the keyword set's size and sortedness, that
  `region_count` does **not** highlight as `region`, and the README/`.el`
  agreement. Needs **no `wolf` and no server**, so it runs anywhere.
- **Server lane** (`tests/server-test.el`): a live `wolf lsp` — one connection,
  eight `ert-info` sections. Publish on open, hover with an exact 3-character
  range, `documentSymbol` finding `main`, a `codeAction` that is answered rather
  than invented, a byte-stable format, an edit/republish/undo/republish
  round-trip, a second document opened mid-session, and a real `shutdown`/`exit`
  handshake. Skips **loudly** with exit 77 and a reason when there is no `wolf`
  at the pin (ls00 §3).

  **One test rather than six**, deliberately: every `eglot` call spawns its own
  server and the capture proxy writes one transcript per spawn, so six granular
  tests would leave six transcripts of which five are discarded. One session
  across the whole surface is both the better recording and the closer model of
  how anyone uses the editor.
- **In the harness**: the profile validates, `lspconf onetruth` runs all 10
  samples **under the emacs profile**, and `lspconf fuzz --profile=emacs` puts a
  long edit session through this client's shape.
- **Derivation**: `cargo xtask emacs-check` re-derives the keyword list from the
  pinned grammar and compares it to the markers in `wolf-mode.el`.

```sh
emacs --batch -l clients/emacs/tests/mode-test.el -f ert-run-tests-batch-and-exit
WOLF_BIN=/path/to/wolf \
  emacs --batch -l clients/emacs/tests/server-test.el -f ert-run-tests-batch-and-exit
```

## Recording the transcript

eglot resolves its server from `eglot-server-programs` by name, so a script
named `wolf` earlier on `PATH` captures everything with no instrumented build
and no change to `wolf-mode.el`:

```sh
# $SHIM/wolf, chmod +x
#!/bin/sh
if [ "$1" = "lsp" ]; then
  cd "$WOLF_LSP_ROOT/vendor/upstream/samples" || exit 1
  exec "$WOLF_LSP_ROOT/target/debug/lspconf" capture \
    --name emacs/smoke --profile emacs --workspace vendor/upstream/samples \
    -- "$WOLF_REAL" lsp
fi
exec "$WOLF_REAL" "$@"
```

```sh
WOLF_LSP_ROOT=$PWD WOLF_BIN=$SHIM/wolf PATH="$SHIM:$PATH" \
  emacs --batch -l clients/emacs/tests/server-test.el -f ert-run-tests-batch-and-exit
```

The recorded session is the test suite, so **every assertion runs while the
session is being recorded**. There is **no `.lsps` beside it**, and that is the
point — no script decided what eglot sent. `lspconf replay
transcripts/emacs/smoke.jsonl` runs it against a live server (10 of the 23
records are deterministically matchable).

**Open one document at a time.** Visiting both samples before connecting makes
eglot send both `didOpen`s in one burst, and under `--batch` that raced badly
enough to fail about one run in seven. The server is not the cause — driven
directly, two back-to-back `didOpen`s produce two publishes in order, every
time — so the test opens `errors.lu` in its own section, which is also what a
person does.
