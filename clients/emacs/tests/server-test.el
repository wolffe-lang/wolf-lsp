;;; server-test.el --- The eglot lane, against a real `wolf lsp' -*- lexical-binding: t; -*-

;; SPDX-License-Identifier: GPL-3.0-or-later

;;; Commentary:

;; A real eglot session against a real `wolf lsp', driven by `emacs --batch'.
;; This is also the recipe that RECORDS `transcripts/emacs/smoke.jsonl': put a
;; capture shim named `wolf' earlier on `PATH' and every assertion below runs
;; while the session is being recorded.  A transcript of a broken session is
;; worse than none, because it replays green forever.
;;
;;   WOLF_BIN=/path/to/wolf emacs --batch -l clients/emacs/tests/server-test.el \
;;         -f ert-run-tests-batch-and-exit
;;
;; With no `wolf' resolvable it **skips loudly** and exits 77, matching the
;; harness convention (ls00 §3) rather than passing quietly.
;;
;; The whole lane is ONE `ert-deftest' holding ONE connection, sectioned with
;; `ert-info'.  That is not laziness about isolation: each `eglot' call spawns
;; its own server, the capture proxy writes one transcript per spawn, and six
;; granular tests therefore leave six transcripts of which five are discarded.
;; One session that exercises the whole surface is both the better recording and
;; the closer model of how anyone actually uses the editor; `ert-info' is what
;; keeps a failure naming its own section.

;;; Code:

(require 'ert)
(require 'eglot)
(require 'jsonrpc)

(defconst wolf-test-root
  (expand-file-name "../../.." (file-name-directory (or load-file-name buffer-file-name)))
  "The repository root, computed from this file so the checkout can live anywhere.")

(add-to-list 'load-path (expand-file-name "clients/emacs" wolf-test-root))
(require 'wolf-mode)

(defconst wolf-test-samples
  (expand-file-name "vendor/upstream/samples" wolf-test-root))

(defvar wolf-test-publishes nil
  "Every `textDocument/publishDiagnostics' seen, newest first, as (FILE . COUNT).")

(cl-defmethod eglot-handle-notification :after
  (_server (_method (eql textDocument/publishDiagnostics)) &key uri diagnostics
           &allow-other-keys)
  (push (cons (file-name-nondirectory uri) (length diagnostics)) wolf-test-publishes))

(defun wolf-test--server ()
  "Resolve a `wolf' the way the harness does: $WOLF_BIN, then `PATH'."
  (let ((from-env (getenv "WOLF_BIN")))
    (cond ((and from-env (file-executable-p from-env)) from-env)
          ((executable-find "wolf") "wolf"))))

;; The loud skip.  Emitted at LOAD time, before ERT has a chance to report a
;; green run over zero exercised assertions.
(let ((server (wolf-test--server)))
  (unless server
    (princ "SKIP: emacs server lane has no wolf binary ($WOLF_BIN, PATH both empty) \
— wolf-lang publishes no release artifact yet; see README `Running the server lane locally'\n")
    (kill-emacs 77)))

(defconst wolf-test-timeout 15.0
  "Wall-clock ceiling for anything this file waits on.")

(defun wolf-test--wait-until (predicate &optional what)
  "Pump the event loop until PREDICATE returns non-nil, or fail after a timeout.

Waiting on a CONDITION with a wall-clock deadline, rather than pumping a fixed
number of times, is not a refinement — it is the difference between a test that
passes and one that passes usually.  `accept-process-output' returns as soon as
any process has output, so `(dotimes (_ 60) (accept-process-output nil 0.05))'
is sixty EVENTS and not three seconds; on a run where the server is chatty it
elapses in milliseconds and the notification being waited for has not arrived
yet.  That produced an intermittent failure of the open-publish assertion in
roughly one run in three."
  (let ((deadline (+ (float-time) wolf-test-timeout)))
    (while (and (not (funcall predicate)) (< (float-time) deadline))
      ;; BOTH calls are load-bearing, and finding that out cost an
      ;; intermittent failure in roughly one run in seven.
      ;;
      ;; `accept-process-output' reads bytes off the connection. It does NOT
      ;; reliably run timers — and jsonrpc.el does not handle a message in its
      ;; process filter: `jsonrpc--process-filter' schedules the actual dispatch
      ;; with `run-at-time' / `timer-activate' (jsonrpc.el, "we all this
      ;; processing in top-level loops timer"). So a run could read every byte
      ;; of a `publishDiagnostics' and never call `eglot-handle-notification'.
      ;;
      ;; `sit-for' is what runs the expired timers. In `--batch' there is no
      ;; input to interrupt it, so it waits the full interval and flushes the
      ;; queue.
      (accept-process-output nil 0.01)
      (sit-for 0.02))
    (unless (funcall predicate)
      (ert-fail (format "timed out after %.0fs waiting for %s"
                        wolf-test-timeout (or what "a condition"))))))

(defun wolf-test--publishes-for (file)
  "How many `publishDiagnostics' notifications have named FILE."
  (seq-count (lambda (p) (equal (car p) file)) wolf-test-publishes))

(defun wolf-test--request (method params)
  (jsonrpc-request (eglot-current-server) method params))

(ert-deftest wolf-eglot-smoke ()
  "One connection, the whole served surface, every assertion on the wire."
  (let* ((default-directory wolf-test-samples)
         (wolf-test-publishes nil)
         (eglot-sync-connect 30)
         (eglot-autoshutdown nil)
         ;; Without this, the explicit `exit' at the end of the session makes
         ;; eglot RECONNECT ("unexpected server exit"), which spawns a second
         ;; server — and under the capture shim that second server overwrites
         ;; the transcript the first one just produced, truncating the recording
         ;; to a handshake.
         (eglot-autoreconnect nil)
         (hello (find-file-noselect (expand-file-name "hello.lu" wolf-test-samples)))
         ;; `errors.lu' is opened LATER, inside its own section, and not here.
         ;; Visiting it up front makes eglot send both `didOpen's in one burst at
         ;; connect, and in `--batch' that raced: the second publish could be
         ;; dispatched while the first was still queued behind jsonrpc's timer,
         ;; and the wait for hello.lu's publish would expire. The server is not
         ;; the cause — driven directly, two back-to-back `didOpen's produce two
         ;; publishes in order, every time. Opening one file at a time is also
         ;; what a person does.
         broken
         server)
    (unwind-protect
        (with-current-buffer hello
          (ert-info ("connect")
            ;; NOT `eglot-ensure': it defers the connection onto
            ;; `post-command-hook', and `--batch' has no command loop, so the
            ;; hook never fires and the session silently never starts — a green
            ;; run over zero exercised assertions.
            (eglot 'wolf-mode (eglot--current-project)
                   'eglot-lsp-server (list (wolf-test--server) "lsp") '("wolf"))
            (setq server (eglot-current-server))
            (should server))

          (ert-info ("diagnostics arrive on open, with no save and no edit")
            ;; eglot sends `didSave' only when the user saves.  A server that
            ;; waited for one would look completely dead here.
            (wolf-test--wait-until
             (lambda () (assoc "hello.lu" wolf-test-publishes))
             "the open publish for hello.lu")
            (should (= 0 (cdr (assoc "hello.lu" wolf-test-publishes)))))

          (ert-info ("hover carries an exact range")
            (goto-char (point-min))
            (search-forward "who")
            (backward-char 1)
            (let ((hover (wolf-test--request :textDocument/hover
                                             (eglot--TextDocumentPositionParams))))
              (should hover)
              (should (plist-get hover :contents))
              (let* ((range (plist-get hover :range))
                     (start (plist-get range :start))
                     (end (plist-get range :end)))
                ;; `who' is three characters — a whole-line range would pass a
                ;; "hover works" check and be wrong.
                (should (= (plist-get start :line) (plist-get end :line)))
                (should (= 3 (- (plist-get end :character)
                                (plist-get start :character)))))))

          (ert-info ("documentSymbol finds main")
            (let ((symbols (wolf-test--request
                            :textDocument/documentSymbol
                            (list :textDocument (eglot--TextDocumentIdentifier)))))
              (should (> (length symbols) 0))
              (should (seq-find (lambda (s) (equal (plist-get s :name) "main")) symbols))))

          (ert-info ("codeAction on a clean file answers without inventing a fix")
            ;; The response may legitimately be empty; what is asserted is that
            ;; the request is answered rather than erroring.  `seqp', not
            ;; `listp': jsonrpc decodes a JSON array to a VECTOR, so an empty
            ;; result is `[]' and `(listp [])' is nil.
            (should (seqp (wolf-test--request
                            :textDocument/codeAction
                            (list :textDocument (eglot--TextDocumentIdentifier)
                                  :range (list :start (list :line 0 :character 0)
                                               :end (list :line 0 :character 0))
                                  :context (list :diagnostics []))))))

          (ert-info ("formatting is byte-stable on canonical input")
            ;; A response that returned a no-op edit instead of NO edits would
            ;; mark every formatted buffer modified and burn an undo state per
            ;; format.
            (should (= 0 (length (wolf-test--request
                                  :textDocument/formatting
                                  (list :textDocument (eglot--TextDocumentIdentifier)
                                        :options (list :tabSize 4 :insertSpaces t)))))))

          (ert-info ("an unsaved edit republishes, and undoing it republishes clean")
            ;; `eglot--signal-textDocument/didChange' is called by hand because
            ;; eglot flushes pending changes from an IDLE timer
            ;; (`eglot-send-changes-idle-time', 0.5 s) and `--batch' never goes
            ;; idle — the notification would sit in the queue forever and the
            ;; test would fail for a reason with nothing to do with wolf.
            ;; Interactively no such call exists and none is needed.
            (let ((before (length wolf-test-publishes))
                  (broken-text "fn broken( {\n"))
              (goto-char (point-max))
              (insert broken-text)
              (eglot--signal-textDocument/didChange)
              (wolf-test--wait-until
               (lambda () (> (length wolf-test-publishes) before))
               "the republish after a breaking edit")
              (should (> (cdar wolf-test-publishes) 0))
              (let ((after-break (length wolf-test-publishes)))
                (delete-region (- (point-max) (length broken-text)) (point-max))
                (eglot--signal-textDocument/didChange)
                (wolf-test--wait-until
                 (lambda () (> (length wolf-test-publishes) after-break))
                 "the republish after undoing it")
                (should (= 0 (cdar wolf-test-publishes)))))
            (set-buffer-modified-p nil))

          (ert-info ("a second document in the same session publishes too")
            (setq broken (find-file-noselect
                          (expand-file-name "errors.lu" wolf-test-samples)))
            (with-current-buffer broken
              (should (eq major-mode 'wolf-mode))
              (eglot--maybe-activate-editing-mode))
            (wolf-test--wait-until
             (lambda () (assoc "errors.lu" wolf-test-publishes))
             "the open publish for errors.lu"))

          (ert-info ("shutdown then exit, and the server answers the shutdown")
            ;; Sent explicitly rather than through `eglot-shutdown': that
            ;; function waits on a process sentinel, and in `--batch' the
            ;; sentinel does not run, so it gives up and SIGKILLs — which would
            ;; record a session that ends in a kill rather than in the
            ;; handshake eglot performs interactively.
            (should (null (jsonrpc-request server :shutdown nil :timeout 10)))
            (jsonrpc-notify server :exit nil)
            ;; Wait for the process to actually go, so the capture proxy has
            ;; flushed its transcript before this process exits.
            (wolf-test--wait-until
             (lambda () (not (jsonrpc-running-p server)))
             "the server to exit after `exit'")))
      (when server (ignore-errors (jsonrpc-shutdown server nil)))
      (dolist (b (list hello broken))
        (when (and b (buffer-live-p b))
          (with-current-buffer b (set-buffer-modified-p nil))
          (kill-buffer b))))))

(provide 'server-test)
;;; server-test.el ends here
