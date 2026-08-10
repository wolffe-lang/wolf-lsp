# Releasing a client

> **No client has ever been released, and today none can be.** Steps 7 and 8
> need a registered publisher, a mirror repository and a published `wolf`;
> [`DISTRIBUTION.md`](DISTRIBUTION.md) lists exactly which human acts are
> missing. Everything before them runs now, on every push.

```sh
cargo xtask release-check
```

That command **is** this document. Every step below is a line in its output,
and each line is one of three things:

| | meaning |
|---|---|
| `PASS` | checked, here, just now |
| `FAIL` | a real problem. Non-zero exit; `cargo xtask ci` goes red |
| `PENDING` | cannot be checked from this repository. Names the human act that would clear it |

**`PENDING` is not a pass, and it never disappears.** A checklist whose
unrunnable steps quietly vanish from the output shrinks until it certifies
nothing — the same failure `MATRIX.md` was built to prevent one layer up. So the
pending rows print on every run, including the ones that will be pending for
months.

`release-check` runs in `cargo xtask ci`, not only at a tag. A checklist first
executed on release day is a checklist whose first execution is a discovery.

---

## The order, and why it is the order

Each step is a precondition for the next. Running them out of order produces a
release that was verified against something other than what shipped.

### 0. There is a `wolf` to be compatible *with*

`PENDING`, and everything else hangs off it. `wolf-lang` tags no release, so the
pin in `vendor/upstream/PIN` is a private-repo sha — not something a user can
acquire. Until wolf-lang s66 publishes an artifact, "install the extension" has
no coherent second half.

*Clears when:* `gh release list --repo tenseleyFlow/wolf-lang` is non-empty.

### 1. Bump the pin, re-vendor, in its own commit

`cargo xtask vendor-check`, plus a cross-check that the recorded submodule
gitlink agrees with `PIN`. The re-vendored `spec/grammar.ebnf` and samples land
in that same commit and nothing else does: a pin bump mixed with other work is a
pin bump nobody can revert.

### 2. Regenerate every derived inventory

`grammar-drift` (VS Code's four generated files), `nvim-check`
(`syntax/wolf.vim` keywords and `pin.lua`), `config-check` (Helix, Zed and the
shared formatter numbers), `emacs-check` (the derived keyword list). Commit any
diff **with the pin**, not after it.

Five independent tables are derived from one pinned grammar, in five target
languages. The redundancy is the check: a drift in one is a drift in all five.

### 3. The conformance suite, green, on all three tier-1 OSes

Split honestly in two, because half of it can run without a server and half
cannot:

- `lspconf verify` — transcripts parse, validate and canonicalise. Runs
  everywhere, gates today.
- `lspconf --require-server replay` and `--require-server onetruth` — the
  transcript library and D34's falsifiable claim (`publishDiagnostics` ==
  `conform-run`). `PENDING` while `lspconf doctor` reports SERVER UNAVAILABLE.
- The **three-OS** claim is CI's, never a local run's (D35). `release-check`
  reports it `PENDING` on principle: it ran on one host, and one host cannot
  substantiate three.

### 4. Every T1 matrix row green

T1 breakage **blocks**. T2 files an issue and proceeds. T3 gets its manual
verification and its stamp now, before the tag rather than after.

`release-check` reads [`MATRIX.md`](MATRIX.md) rather than trusting it: every
transcript and profile a T1 or T2 row cites must exist on disk, and any row
stamped `NEVER` is reported as the unverified row it is. Zed is stamped `NEVER`
today and will be until somebody runs it.

### 5. Refresh the stamps

`MATRIX.md`'s "last reviewed against wolf pin" line must name the current pin —
checked. Then `COMPAT.md`, which is generated: `cargo xtask compat-generate`
rewrites the table and the two client artifacts from `clients/*/compat.json`.

**`max_tested` moves only on step 3's evidence.** `compat-check` derives the
earned set from the pin and fails on any declared range wider than it. That gate
is the whole point of [`COMPAT.md`](COMPAT.md) and it was exercised red on
purpose (see that file, §The red test).

### 6. A changelog per client

User-visible changes only. A client changelog that recites internal refactors
trains users to stop reading it; internal churn belongs in the campaign
closeout. `release-check` fails if a `clients/*/CHANGELOG.md` is missing or if
its first `##` heading does not name the version that client's `compat.json`
declares.

### 7. Tag, and let the publish job run

Tag pattern `client-<name>-v<semver>`, which cannot collide with a wolf-lang
release tag.

| | |
|---|---|
| 7a. VS Code Marketplace | `PENDING` — the pipeline is dry-run proven (`vsce package` + `vsce ls` against a reviewed contents list + the manifest lint, CI job `vscode-package`). The publisher is an unregistered placeholder, so no token exists and no publish is possible |
| 7b. Open VSX | `PENDING` — same vsix, `ovsx publish`. No namespace registered |
| 7c. nvim mirror split | **checked.** `git subtree split --prefix=clients/nvim`, then every file the mirror needs asserted present and nothing from the harness leaked. Proven end to end into a scratch path with local-only remotes, including a real Neovim loading the cloned mirror |
| 7d. nvim mirror push | `PENDING` — `tenseleyFlow/wolf.nvim` does not exist. **Its default branch must be `main` before the first push**, or every clone is empty; see `DISTRIBUTION.md` §neovim |

Note what 7c reports on a dirty working tree: `git subtree split` reads
**history**, so on uncommitted changes it faithfully splits HEAD — a correct
answer to a question nobody asked. That case is reported as `PENDING` naming the
uncommitted files, never as a pass. In CI, where the tree is always clean, it is
a hard gate.

### 8. Install it on a clean machine, per T1 editor

`PENDING`, and unreachable by construction: it installs *from the published
channel*, and step 7 has not run.

Install from the marketplace / the mirror on a machine with no `wolf-lsp`
checkout, open a vendored sample, **see a diagnostic**, stamp the matrix row. A
release nobody installed is a release nobody has tested, and every other step in
this document is a proxy for this one.

### 9. Refresh the upstream statuses

[`UPSTREAM.md`](UPSTREAM.md) states every patch's status in a five-word
vocabulary, and `release-check` fails if a row uses none of them. It **cannot**
check that a row is true — nothing here can observe a PR moving — so 9b is
permanently `PENDING`: open each link and re-read the state before tagging.

---

## Reading the output

Today, on a clean tree, `release-check` prints roughly:

```
13 checked, 0 failed, 9 pending a human action.
```

The pending count is the interesting number, and it should shrink for reasons
somebody can name. Most of it collapses the moment step 0 clears: 3b, 3d, 7a,
7b, 7d and 8 are all waiting, directly or transitively, on a `wolf` a stranger
can install.
