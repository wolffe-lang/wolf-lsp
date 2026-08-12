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
  '("bool" "int" "str" "Self")
  "Type names that are NOT reserved words, so they live outside the markers.
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
