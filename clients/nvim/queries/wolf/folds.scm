;; folds.scm — foldable blocks and items.
;;
;; STILL EMPTY. Written when tree-sitter-wolf was scaffold-only (no grammar,
;; so no node names, and a guessed pattern would have raised "invalid node
;; type" for every user the day a real grammar landed). The grammar has
;; existed since le02 — helix and zed pin it at 1834e73 (tl16) — and nobody
;; has yet written this file against its node names; that is a lane of its
;; own. `:checkhealth wolf` reports this file's compiled pattern count, so
;; zero is a state you can see rather than one you discover.
;;
;; The regex fallback in `syntax/wolf.vim` is the highlighting story until
;; then, and it is derived from the same pinned EBNF.
;;
;; Full reasoning and the filling order: ../README.md
