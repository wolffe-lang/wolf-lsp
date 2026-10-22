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
