# tl32 — editor grammar revisions

## Prediction (before either pin moves)

Helix and Zed move from tree-sitter-wolf
`e7388233aaba4165b9c9f9d0165b9566d1eba1dc` to
`936a96f565be29b871bb8a94806e49a9b1b33c33`, the merged #27 fix.

The six captured LSP smokes replay against the same wolf server and are not
parser captures. Their records should not move; replay should match emacs 10,
fackr 9, facsimile 7, helix 9, nvim 15, and vscode 30 records. Their headers
also name the wolf pin, not this grammar revision, so none should need a
header restamp or re-capture.

The highlight witness is a `.lu` file with a line comment after a newline.
At `e738823`, the external scanner consumes the terminator so the comment
capture begins at the preceding line's end. At `936a96f`, the newline is its
own external token and the comment begins at the `//` on the following line.
The comment remains a comment highlight; its start position is what changes.

## Result (kasumi, linux x86-64, the acquired wolf 0.2.34, sha256 88a9dd3d…)

- Witness: `tree-sitter-wolf/test/highlight/comments.lu` with the tree-sitter-wolf
  `queries/highlights.scm` (byte-identical to Zed's shipped copy), parsed at each
  rev. At `e738823` the second comment of the file is captured `comment.line
  1:74-2:70`, starting at the previous line's end and holding the newline; at
  `936a96f` it is `2:0-2:70`, starting at its own slashes. The comment stays
  `comment.line`; only the range moves.
- Smokes: `lspconf --require-server replay` against the acquired server, 79 ok,
  0 mismatched, nothing skipped. emacs 10, fackr 9, facsimile 7, helix 9, nvim 15,
  vscode 30 records, teardowns as recorded. No record moved; none re-captured; no
  transcript changed (these are server sessions, not parser captures).
- Plant (local, R2): the helix rev one digit off (`…c32`) turns
  `xtask config-check` red: `config-check: grammar pin drift:
  clients/helix/languages.toml pins tree-sitter-wolf at 936a96f565be29b871bb8a94806e49a9b1b33c32
  but clients/zed/extension.toml pins 936a96f565be29b871bb8a94806e49a9b1b33c33`.
  Restored; `xtask ci` then exits 0.
