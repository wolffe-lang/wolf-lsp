# Transcripts

Recorded JSON-RPC sessions, one directory per client, replayed against the real
`wolf lsp` by `lspconf replay`.

Empty at ls00, necessarily: there is no server to record. Transcripts are
*captured* from a running client through `lspconf record` (ls01 §1) — never
hand-written, because a hand-written approximation of a client's behaviour
tests the approximation.

The format is frozen (ls00 §2) and its parser, matchers, and normalization are
tested against hand-written fixtures under
`crates/lsp_transcript/tests/fixtures/`. Those fixtures are format exercises,
not sessions, and they never move here.
