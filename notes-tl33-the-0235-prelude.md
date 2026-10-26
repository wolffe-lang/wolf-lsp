# tl33 — the 0.2.35 prelude data (wolf-lsp#56, merged at df8c80a0; rebased and repaired by tl33b)

Materialised on trunk from the PR body by the orchestrator at the merge (a Codex lane writes no note file).

## Forbidden
- No merge, tag, release, issue action, local Mac build, attribution trailer, or unreleased wolf-lang#904 names.

## Inputs verified
- wolf-lang `v0.2.35` (`cf0c13d`) `spec/prelude.json`, SHA-256 read twice: `88d0864a15dc453bd48301dc41489f5638df28c4fe2cf9e3f4ad366914ab10bf`.
- Thirteen functions added; no mark or type added.

## Prediction
- The vendored prelude has all thirteen released names and excludes `env_arg0` / `env_args_bytes`; the compiler owns hover, signature, and completion behavior.

## Evidence index
- `docs/PIN-0235.md`
- `vendor/upstream/spec/prelude.json`
- `xtask/src/prelude_drift.rs`
- tl33b: clean rebase onto `origin/trunk` (`4e2c1f5`). Run `39968654226` had two distinct failures: VS Code job `120465249678` expected `wolf 0.2.34 (wolfgang, pin 9da7bdc)` instead of the installed `wolf 0.2.35 (wolfgang, pin cf0c13d)`; test job `120465249790` failed rustfmt on the unreleased-name assertion before running tests. Both repaired.
- Both generated client pin files and all five compatibility declarations now match 0.2.35; derived compatibility outputs kept consistent. The compatibility evidence cites tl33's existing hasu measurements, not a new editor capture.
- Prelude rechecked byte-for-byte against the tagged spec; SHA-256 read twice and unchanged. Exactly the thirteen function names in `docs/PIN-0235.md` added, none removed; no new type or mark.
- README read, current. No local build or test; CI remains definitive.

## Done-when
- CI pin, vendor, and transcript gates pass at this head. The two wolf-lang#904 names are left for the next pass.

- `89b4132` docs: predict the 0.2.35 prelude update
- `afae88a` upstream: pin 0.2.35 prelude data
- `d7e56d6` xtask: cover 0.2.35 prelude names
- `05ab708` vendor: restore 0.2.35 pin record
- `b2c30a1` transcripts: re-record at 0.2.35
- `beb3bc2` docs: record 0.2.35 transcript evidence
- `f45c2d0` vendor: correct 0.2.35 release id
- `03e7178` tests: format the unreleased prelude assertion
- `df8c80a` tests: the 0.2.35 pin string

### Test checklist
- [x] Source prelude compared as name/kind sets.
- [x] Hasu environment read (tl33).
- [x] Pinned `cargo fmt --all -- --check` (tl33b, no compilation).
- [x] Static comparison: both pin renderers and both compatibility code outputs exactly match their Rust generator templates; all five compatibility declarations agree with PIN.
- [x] `git diff --check`.
- [ ] CI `tier pr`.
- [ ] CI `tier full`.

Footprint: cross-cutting

Correction: `421239566` in the initial PIN was a tl33 manual transcription error; release API verification gives `421613621`.


