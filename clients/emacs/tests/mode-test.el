;;; mode-test.el --- The binary-free emacs lane -*- lexical-binding: t; -*-

;; SPDX-License-Identifier: MIT OR Apache-2.0

;;; Commentary:

;; Everything here runs with **no `wolf' binary and no server**, which is what
;; keeps this lane from being dark on a CI runner (ls00 §3).  ERT ships with
;; Emacs, so the lane has no dependency beyond `emacs --batch'.
;;
;;   emacs --batch -l clients/emacs/tests/mode-test.el \
;;         -f ert-run-tests-batch-and-exit

;;; Code:

(require 'ert)

(defconst wolf-test-root
  (expand-file-name "../../.." (file-name-directory (or load-file-name buffer-file-name)))
  "The repository root, computed from this file so the checkout can live anywhere.")

(add-to-list 'load-path (expand-file-name "clients/emacs" wolf-test-root))
(require 'wolf-mode)

(defun wolf-test--in-temp-lu (contents body)
  "Run BODY in a real `.lu' file containing CONTENTS.
A temp *buffer* would not exercise `auto-mode-alist', which is half of
what this file is checking."
  (let ((path (make-temp-file "wolf-test" nil ".lu" contents)))
    (unwind-protect
        (with-current-buffer (find-file-noselect path) (funcall body))
      (delete-file path))))

(ert-deftest wolf-a-dot-lu-file-gets-wolf-mode ()
  (wolf-test--in-temp-lu "fn main() -> !int { 0 }\n"
                         (lambda () (should (eq major-mode 'wolf-mode)))))

(ert-deftest wolf-mode-derives-from-prog-mode ()
  ;; Not decoration: every `prog-mode-hook' package in a user's config —
  ;; `display-line-numbers-mode', `flymake', `electric-pair-mode' — keys off
  ;; this and off nothing else.
  (should (provided-mode-derived-p 'wolf-mode 'prog-mode)))

(ert-deftest wolf-dot-wolfi-is-deliberately-not-mapped ()
  ;; `wolfi' v0 is a BINARY format and `wolf lsp' discovers modules by `.lu'
  ;; alone, so a `.wolfi' buffer must not land in a mode eglot manages.  A
  ;; regression here produces a buffer that looks supported and is not.
  (should-not (assoc "\\.wolfi\\'" auto-mode-alist))
  (should-not (eq (cdr (assoc-default "x.wolfi" auto-mode-alist #'string-match-p))
                  'wolf-mode)))

(ert-deftest wolf-comment-syntax-is-line-only ()
  (wolf-test--in-temp-lu "// a comment\nfn main() -> !int { 0 }\n"
   (lambda ()
     (should (equal comment-start "// "))
     (should (equal comment-end ""))
     ;; **Wolf has no block comment.**  `comment-end' being empty is the
     ;; machine-readable form of that; `newcomment.el' would otherwise offer
     ;; `comment-region' a construct the lexer rejects.
     (goto-char (point-min))
     (should (nth 4 (syntax-ppss (+ (point) 3))))
     (goto-char (point-min))
     (forward-line 1)
     (should-not (nth 4 (syntax-ppss (+ (point) 3)))))))

(ert-deftest wolf-apostrophe-does-not-open-a-string ()
  ;; Wolf has no character literal, so `'' is punctuation.  If it were a string
  ;; delimiter, every "don't" in a comment would paint the rest of the file.
  (wolf-test--in-temp-lu "// don't panic\nfn main() -> !int { 0 }\n"
   (lambda ()
     (goto-char (point-max))
     (should-not (nth 3 (syntax-ppss))))))

(ert-deftest wolf-indent-and-width-are-the-formatters ()
  ;; `wolf_fmt::doc::INDENT' = 4, `WIDTH' = 100, spaces only.
  (wolf-test--in-temp-lu "fn main() -> !int { 0 }\n"
   (lambda ()
     (should (= tab-width 4))
     (should (= fill-column 100))
     (should-not indent-tabs-mode))))

(ert-deftest wolf-the-keyword-list-is-the-closed-set-of-fifty ()
  ;; The *contents* are enforced against the pinned grammar by `cargo xtask
  ;; emacs-check'; the count is the spec's own checksum and is cheap to assert
  ;; from inside Emacs as well, so a bad hand-edit fails in both lanes.
  (should (= (length wolf-mode-keywords) 50))
  (should (equal wolf-mode-keywords (sort (copy-sequence wolf-mode-keywords) #'string<)))
  (should-not (seq-intersection wolf-mode-keywords wolf-mode-builtin-types)))

(ert-deftest wolf-keywords-highlight-and-identifiers-do-not ()
  (wolf-test--in-temp-lu "fn main() -> !int {\n    let region_count = 1\n    0\n}\n"
   (lambda ()
     (font-lock-ensure)
     (goto-char (point-min))
     (should (eq (get-text-property (point) 'face) 'font-lock-keyword-face))
     ;; `region_count' merely CONTAINS `region'.  `regexp-opt' with the
     ;; `symbols' mode is what keeps that from colouring, and this is the
     ;; assertion that would catch someone "simplifying" it away.
     (search-forward "region_count")
     (should-not (eq (get-text-property (- (point) 2) 'face) 'font-lock-keyword-face)))))

(ert-deftest wolf-the-readme-snippet-is-this-file-verbatim ()
  ;; `wolf-mode.el' is shipped as a FILE so CI can run it, and documented as a
  ;; SNIPPET so a reader can paste it.  Those are two copies of one artifact,
  ;; and this is what stops them drifting.
  (let ((el (with-temp-buffer
              (insert-file-contents
               (expand-file-name "clients/emacs/wolf-mode.el" wolf-test-root))
              (buffer-string)))
        (readme (with-temp-buffer
                  (insert-file-contents
                   (expand-file-name "clients/emacs/README.md" wolf-test-root))
                  (buffer-string))))
    (should (string-search (string-trim-right el) readme))))

(provide 'mode-test)
;;; mode-test.el ends here
