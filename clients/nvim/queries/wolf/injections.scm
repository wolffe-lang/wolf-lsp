;; injections.scm — 're"…"' bodies as regex, and '{…}' f-string interpolations back into wolf.
;;
;; INTENTIONALLY EMPTY. tree-sitter-wolf is scaffold-only (licenses and a
;; README, no grammar), so there are no node names to write patterns against,
;; and a guessed pattern would raise "invalid node type" for every user the day
;; a real grammar lands. `:checkhealth wolf` reports this file's compiled
;; pattern count, so zero is a state you can see rather than one you discover.
;;
;; The regex fallback in `syntax/wolf.vim` is the highlighting story until
;; then, and it is derived from the same pinned grammar this parser will be.
;;
;; Full reasoning and the filling order: ../README.md
