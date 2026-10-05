# Rulings #38 and #39 in the editor layer (tl17, 2026-10-05)

Two rulings from the maintainer's batch of 2026-10-05, both on issues this repo
filed against itself:

- **#38 = C** (wolf-lsp#17): `lspconf replay` accepts an optional trailing
  helix `shutdown` and reports which shape it saw. One transcript.
- **#39 = B** (wolf-lsp#32), the editor half: `type-names-check` reads wolf's
  published prelude list instead of the second segment of the spec's `type.*`
  anchors.

The prediction was committed before any change (`0afe3b9`). Every measurement
below ran on kasumi against the acquired `wolf 0.2.20` linux archive (sha256
`24855d5e…`, `wolf` member `3fb48c1d…`), which is today's pin, `cdde128`.
The logs are under `kasumi:~/lanes/tl17/` and their digests are in the table
at the end.

## The pin is 0.2.20, and it has no list

The contract assumed 0.2.23. `vendor/upstream/PIN` is `cdde128`, which is
`wolf 0.2.20`, pinned by tl16. Either way, no release carries the list:
wolf-lang added it at `6b5db762` (s212, `v0.2.23-51-g6b5db762`), after 0.2.23
was cut. The pinned binary answers:

```
$ wolf prelude --json
wolf: `prelude` is not a wolf command.
(`wolf --help` lists every command and shows a first program)
rc 2
```

So at today's pin the gate has neither source. It says so on its summary line
and keeps trunk's anchor-based behaviour. Nothing moved the pin: #39 does not
need a pin move to land the reader, only to switch it on.

## #39: the candidate set is the published list

**Source order** (`xtask/src/type_names.rs::choose_candidates`):

1. `vendor/upstream/spec/prelude.json`. If the pinned compiler also answers
   `wolf prelude --json`, the two must be the same bytes. A vendored file with
   a compiler that has no `prelude` command is refused, because the file is
   from a newer wolf-lang than the pin.
2. Otherwise, the pinned compiler's `wolf prelude --json`.
3. Otherwise, the anchor index, but only when the compiler's refusal is
   exactly "`prelude` is not a wolf command". Any other failure is red. The
   gate prints `NO PRELUDE LIST AT THIS PIN … (wolf-lsp#32)` whenever it falls
   back.

The reader checks `schema == "wolf-prelude/0"`, ignores unknown keys, and takes
the `names[]` entries whose `kind` is `builtin_type` or `type`. It refuses an
unknown `kind`, because a new kind is not a new field and so would not bump the
schema number. A human decides whether it names a type.

**What the list adds is equality.** The anchor-based gate proves that each name
in the hand lists resolves, and that each anchor word that resolves is listed.
With the list, the gate also proves:

- every name in `TYPE_NAMES` or `TYPE_POSITION_UNPAINTED` is in the published
  list (`Self` excepted, since it is a context alias and not a prelude name);
- every published type name resolves at the pin. A listed name that is E0301
  means the list and the compiler disagree, and that is red.

### The candidate set, old against new (prediction 1: held)

| | count | words |
|---|---|---|
| old: `type.*` second segments | 18 | byte char closure comb err float fn generic interp list map method numlit range row str trait unit |
| new: `builtin_type` + `type` at `6b5db762` | 25 | List Map Pool Mutex channel Scope Proc range · bool str byte char int uint i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 wrapping |
| in both | 4 | byte char range str |
| leave | 14 | closure comb err float fn generic interp list map method numlit row trait unit |
| arrive | 21 | List Map Mutex Pool Proc Scope bool channel f32 f64 i8 i16 i32 i64 int u8 u16 u32 u64 uint wrapping |

The 17 `builtin_type` entries are `TYPE_NAMES` less `Self`, and the 8 `type`
entries are `TYPE_POSITION_UNPAINTED`, both exactly. The unit test
`the_first_published_list_has_twenty_five_type_names_and_they_are_ours` asserts
this against a byte-identical fixture of the list,
`xtask/fixtures/prelude-6b5db762.json` (blob `ce774f97…`, sha256 `fb24125c…`).

### Which `TYPE_POSITION_UNPAINTED` rows become redundant (prediction 2: held)

None of the rows can be removed. Each one carries a paint ruling, and the list
has no field for that. What the list makes redundant is the job of finding
those names: `Scope` and `Proc` were added by hand at 0.2.16 (tl11) because
the gate could not ask about them. Once the list is in force, dropping either
row turns the gate red (G5 below).

One mismatch with upstream that this lane did not act on: the list anchors
`Scope` at `conc.proc.handle`, but the `Scope` row here cites
`[conc.task.scope]`. Both anchors exist at `6b5db762`.

### Red, then green (prediction 3: held)

| run | tree | wolf | result |
|---|---|---|---|
| T1 | trunk `052e240` with the `Scope` row deleted | 0.2.20 | **green**, "18 painted, 7 unpainted, 18 anchor candidates, all classified". Trunk cannot see a missing `Scope` (`b2.log`) |
| G5 | branch with the `Scope` row deleted, compiler answering `6b5db762`'s list | 0.2.20 + list | **red**, rc 1: "wolf's published prelude list names `Scope` as a type and the pinned compiler resolves it, but xtask has not classified it" (`b2.log`) |
| G1 | branch head `4125f1c` | 0.2.20 | green, `NO PRELUDE LIST AT THIS PIN …`, 18 anchor candidates (`b3.log`) |
| G2 | head, compiler answering `6b5db762`'s list | 0.2.20 + list | green, "18 painted, 8 unpainted, 25 prelude-list candidates, all classified" (`b3.log`) |
| G3 | head, compiler answering the list less `Scope` (sha256 `fbeb42c5…`) | 0.2.20 + planted list | **red**, rc 1: "xtask lists `Scope`, and wolf's published prelude list does not name it as a type" (`b3.log`) |
| G4 | head with `6b5db762`'s list vendored | 0.2.20 | **red**, rc 1: "vendored but the pinned compiler has no `prelude` command" (`b3.log`) |
| G4b | the same copy, compiler answering the same list | 0.2.20 + list | green, candidates from the vendored file (`b3.log`) |

"Compiler answering the list" means a wrapper set as `WOLF_BIN` that prints
the file for `prelude --json` and runs the real 0.2.20 binary for every other
command, including `--version` and the probes. G5 and T1 each built their own
copy of the tree, because `xtask` reads its root from `CARGO_MANIFEST_DIR`.

`vendor-check` now compares `spec/prelude.json` with the submodule in both
directions as soon as either side has it. The first pin bump past `6b5db762`
has to vendor it.

## #38: replay names the teardown

`lsp_harness::replay::teardown` reads the shape off a transcript's records:

- **shutdown answered**: emacs, fackr, nvim and vscode, plus every scripted
  transcript;
- **no shutdown**: the session ends on an ordinary record. This is helix's
  usual shape, and facsimile's;
- **a trailing shutdown, unanswered in the capture**: the last record is the
  client's `shutdown` request. This is helix's race;
- **unanswered with records after it**: produced by no tracked client, and
  always refused.

A trailing `shutdown` is accepted only for a client listed in
`profiles::TEARDOWN_RACES`, and only `helix` is listed, with tl03's
measurement as its evidence. Replay sends that last frame live and holds the
answer to LSP's `"result": null`. A client-recorded transcript's replay line
and `verify` line now name its shape. The same 20th record on any other
client is refused by `verify` (exit 1) and by `replay` (exit 2), before the
pin check, so a cut-short capture cannot hide behind a skip.

### Helix's two shapes (prediction 4: held)

The 20-record capture is the committed 19-record `transcripts/helix/smoke.jsonl`
plus tl03's run-B record 20, `{"dir":"c2s","id":7,"kind":"request","method":"shutdown","seq":20}`.
Run B's own file was never committed, and kasumi has no helix, so it could not
be re-captured. Both files were given the header `wolf_pin = cdde128` so that
replay runs them instead of skipping them.

| run | tree | capture | output |
|---|---|---|---|
| trunk | `052e240` | 19 records (sha256 `2f37bcf7…`) | `ok  helix/smoke — 9 record(s) matched` (`trunk.log`) |
| trunk | `052e240` | 20 records (sha256 `b42c6f13…`) | `ok  helix/smoke — 9 record(s) matched`: the **same line**, so the shape is invisible (`trunk.log`) |
| trunk | `052e240` | the 20 records named `nvim/cutshort` (sha256 `c20ecc42…`) | replay `ok`, rc 0, and verify `ok`, rc 0: **accepted** for a client that does not race (`b2.log` T2, T3) |
| head | `4125f1c` | 19 records | `ok  helix/smoke — 9 record(s) matched; teardown: no shutdown, the session ends on an ordinary record` (`b3.log`) |
| head | `4125f1c` | 20 records | `ok  helix/smoke — 9 record(s) matched; teardown: a trailing shutdown, unanswered in the capture (a racing client, wolf-lsp#17; sent live, answered null)` (`b3.log`) |
| head | `4125f1c` | `nvim/cutshort` | **refused**, rc 2: "`nvim` is not a client whose teardown races the capture" (`b3.log`) |

In CI, `tests/teardown.rs` replays both shapes live on all three server-lane
OSes. It takes the committed capture and gives it this pin's header in a
scratch directory. It asserts only the teardown, because whether the 19
records match is the smoke's own claim, and that waits on a re-capture
(wolf-lsp#26).

## At the head (`b3.log`)

fmt clean, clippy `-D warnings` clean, `cargo test --workspace` 249 passed
and 0 failed, with the server present. `vendor-check`, `independence` and
`fixtures-check` all pass. The full `replay` gives 71 ok, 0 mismatched, 6
skipped (the six smokes at `30731a6`, #26), rc 0. `verify` passes, rc 0, and
names the teardown of each of the six smokes.

kasumi's cargo is 1.96.1, but this repo pins 1.97.1, so CI on the pinned
toolchain is the proof of record.

## Logs (kasumi:`~/lanes/tl17/`)

| file | sha256 |
|---|---|
| `setup.log` | `e476f87d…` |
| `trunk.log` | `99f7a929…` |
| `b1.log` | `3f6f3530…` |
| `b2.log` | `b2b0ff50…` |
| `b3.log` | `cb216d78…` |
