# Capability profiles

One client-capability document per real client, **derived from the real
client** in ls02–ls06, each recording the client commit it was read from. A
profile that drifts from its client is a lie the suite tells.

Planned set (ls01 §4): `fackr`, `facsimile`, `nvim`, `vscode`, `helix`, `zed`,
plus synthetic `minimal` (nothing optional declared) and `maximal`.

Empty at ls00 on purpose: a profile invented here rather than read off a
client would be exactly the fiction the file above warns about. `lspconf
verify` validates whatever lands here as soon as it lands.
