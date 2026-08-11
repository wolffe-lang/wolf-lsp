# Distribution — every channel, and the gate on each one

> # NOTHING IN THIS REPOSITORY HAS BEEN PUBLISHED ANYWHERE.
>
> No marketplace listing, no Open VSX namespace, no `wolf.nvim` mirror, no
> registry entry, no tag. Every pipeline below is **built, exercised offline,
> and switched off**; each one stops at a gate that needs a human, a credential,
> or a `wolf` a stranger can install — and none of the three exists today.
>
> This is not caution about polish. `wolf-lang` publishes no release, so
> "install the extension" has no coherent second half: whatever a user installed
> would have no server to talk to. Every channel here is downstream of that one
> fact, and it is step 0 of [`RELEASE.md`](RELEASE.md).

What this file is for: when the gates *do* open, the person opening them should
be reading a rehearsed procedure rather than inventing one. So every section
below states the channel, what has actually been proven about it, the exact
command, and the specific human act that is missing.

## The one thing this repository does distribute

A **per-run vsix artifact**, from CI job `vscode-package`: `npm run test:manifest`
(the publish dry run), `vsce package`, a 1 MiB size gate, and
`actions/upload-artifact` with 14-day retention. It is downloadable from the run
that produced it and traceable to a commit.

That is deliberately not a release channel, and the short retention is there so
nobody mistakes it for one. It has no version discipline, no changelog contract
and no compatibility promise beyond the commit it came from.

---

## VS Code — the marketplace path

### What is proven

`@vscode/vsce` has **no `--dry-run` flag**. `vsce publish` either uploads or it
errors, and the nearest rehearsal, `vsce verify-pat`, needs a real token for a
real publisher. So the dry run is assembled from the three things that run
offline, and between them they cover everything a publish checks before it opens
a socket:

| step | command | what it proves |
|---|---|---|
| package | `npm run package` | the same manifest validation, the same `vscode:prepublish` script and the same file selection a publish performs — ending in a real vsix (318 KB, 216 files) |
| contents | `npm run test:manifest` | `vsce ls`, projected and compared against the reviewed [`PACKAGE-CONTENTS.txt`](../clients/vscode/PACKAGE-CONTENTS.txt). A `.vscodeignore` change or a new runtime dependency is a diff a person reads |
| manifest | `npm run test:manifest` | the listing fields `vsce` does not enforce: required keys, scoped activation, a bundled `LICENSE.md` that still quotes both root licenses verbatim, and `compat.json` travelling inside the artifact |
| install | `npm run install:vsix` | the vsix installs into a throwaway VS Code profile and the editor lists it afterwards (CI job `vscode-extension`) |

What is left after all four is the upload itself.

### The channels

Two, keyed to tags. The pattern is `client-<name>-v<semver>`, which cannot
collide with a `wolf-lang` release tag — a client version is the client's, never
wolf's (ls07 non-target: no version lockstep).

| channel | tag | command |
|---|---|---|
| pre-release | `client-vscode-v0.1.0` on the trunk stream | `vsce publish --pre-release --packagePath wolf-0.1.0.vsix` |
| stable | `client-vscode-v0.2.0` | `vsce publish --packagePath wolf-0.2.0.vsix` |

Pre-release exists so a client can be dogfooded against an unreleased `wolf`
without shipping it to everyone. **VS Code's convention is that pre-release
versions carry an odd minor and stable versions an even one** — the marketplace
has no separate version space, so the parity *is* the channel marker, and a
stable release published with an odd minor cannot be corrected afterwards.

### Open VSX, in the same job

VSCodium, Cursor, Gitpod and Eclipse Theia users are not second-class: the same
vsix goes to both registries in one job, so it is not possible to ship a version
to one and forget the other.

```sh
ovsx create-namespace wolf-lang -p "$OVSX_PAT"   # once, ever
ovsx publish wolf-0.1.0.vsix -p "$OVSX_PAT"
```

`ovsx` takes the identical artifact — no second build, no second manifest.

### The publish job, and why it is not a workflow file

The job below is **prose on purpose**. A `.github/workflows/publish.yml` sitting
in the repository is one accidental tag away from firing, and "the secrets are
not configured" is a weaker guarantee than "the workflow does not exist". The
hard boundary this track was built under is that no step can cross a network
without an explicit human act; committing the workflow would move that boundary
from *impossible* to *unconfigured*.

Add it in the same commit that first publishes, and no earlier:

```yaml
  publish-vscode:
    # The ONLY job allowed to read these secrets, and only on a client tag.
    if: startsWith(github.ref, 'refs/tags/client-vscode-v')
    runs-on: ubuntu-latest
    environment: marketplace          # protection rules + required reviewers
    permissions:
      contents: read
    defaults:
      run:
        working-directory: clients/vscode
    steps:
      - uses: actions/checkout@v5
      - uses: actions/setup-node@v4
        with: { node-version: '22', cache: npm,
                cache-dependency-path: clients/vscode/package-lock.json }
      - run: npm ci
      - run: npm run compile
      # The gate: the same dry run CI already ran, re-run against the tag.
      - run: npm run test:manifest
      - run: npm run package
      - run: npx vsce publish --packagePath wolf-*.vsix
        env: { VSCE_PAT: '${{ secrets.VSCE_PAT }}' }
      - run: npx ovsx publish wolf-*.vsix
        env: { OVSX_PAT: '${{ secrets.OVSX_PAT }}' }
```

Two properties are load-bearing and easy to lose in a later edit: the `if:` on
the tag pattern, and `environment:` — a GitHub environment is what confines the
secrets to this one job and can require a reviewer before it runs.

### OWED TO HUMAN — publisher registration

**Nothing in this repository may perform any of this.** Each item is a credential
or an identity, and a repository that could create one could also leak one.

- [ ] **Register a VS Marketplace publisher.** Create it at
      <https://marketplace.visualstudio.com/manage>, backed by a Microsoft
      account (an Azure DevOps organization is created implicitly). Record the
      publisher ID here when it exists.
- [ ] **Replace the placeholder.** `clients/vscode/package.json` says
      `"publisher": "wolf-lang-unpublished"` — a deliberately invalid-looking
      value, so an accidental publish attempt fails on identity rather than
      succeeding under a name nobody chose. Change it **in the same commit that
      first publishes**, never earlier. `npm run test:manifest` reports it as
      owed until then.
- [ ] **Create the PAT.** Azure DevOps → Personal Access Tokens, organization
      **"All accessible organizations"** (a single-org token is rejected by the
      marketplace API), scope **Marketplace → Manage**. Azure DevOps caps PAT
      lifetime at one year, so this expires — put the expiry date in a calendar,
      because the failure mode is a release that dies at the last step.
- [ ] **Record custody, not just the token.** Who owns the account, where the
      token is stored, and **who can rotate it**. A marketplace listing
      controlled by one unrecoverable credential on one laptop is an outage
      waiting for a disk failure; two people must be able to rotate it.
- [ ] **Store it as `secrets.VSCE_PAT`** in a protected GitHub environment, not
      as a plain repository secret.
- [ ] **Open VSX**: an Eclipse Foundation account, a signed Eclipse Contributor
      Agreement, then `ovsx create-namespace wolf-lang`. Token as
      `secrets.OVSX_PAT`. The ECA is a signature by a person and cannot be
      automated.
- [ ] **Listing content**, which no program can produce:
      - a 128×128 PNG icon (`"icon": "images/icon.png"`) — branding art is a
        design decision, not a build step;
      - `keywords`, which is a decision about what searches this listing should
        win, and guessing them here would be guessing at marketing;
      - **a screenshot of real diagnostics from a real corpus sample.** ls07 §1
        requires a screenshot and forbids a mockup, so it cannot be taken until
        a live `wolf lsp` exists to produce one.

---

## Neovim — the generated mirror

### The decision

`wolf.nvim` lives in this repository under `clients/nvim/`, and is **published to
a standalone mirror repository** built by `git subtree split`.

The alternative — telling users to install a subdirectory — was rejected because
plugin managers handle subdirectories badly and inconsistently, and lazy.nvim
has no option for one at all. A mirror costs one CI step and makes
`{ 'wolffe-lang/wolf.nvim' }` work in every manager.

The mirror is **generated, never hand-edited**. Its README must say so in its
first paragraph, because the mirror is what a contributor finds first and a PR
against a generated repository is work thrown away.

### What is proven, and how

`cargo xtask nvim-split` computes the split and verifies the resulting tree: it
asserts every file the mirror needs is present, and that nothing from the
harness (`crates/`, `vendor/`, `transcripts/`) rode along. It does not push, does
not branch and does not tag.

Beyond that, the whole chain was executed end to end **into a scratch path with
local-only remotes** — no network, no GitHub:

```
git clone --no-local . /tmp/…/repo          # the split source
git subtree split --prefix=clients/nvim     # -> 9cbd2fe, 30 files
git push /tmp/…/wolf.nvim.git 9cbd2fe:refs/heads/main
git push /tmp/…/wolf.nvim.git v0.0.1-scratch
git clone /tmp/…/wolf.nvim.git /tmp/…/wolf.nvim
cd /tmp/…/wolf.nvim && nvim --headless -u tests/minimal.lua -l tests/run.lua
  -> 16 passed, 0 failed
```

So the mirror is not merely "a directory with the right filenames": a real
Neovim loaded the cloned mirror off its runtimepath and the whole plugin suite —
filetype detection, the LSP config, `:checkhealth wolf`, the help tags, and both
ls07 compatibility cases — passed from it.

**One finding, and it is the kind only running the thing produces.** The clone
step printed `warning: remote HEAD refers to nonexistent ref, unable to
checkout` and produced an **empty working tree**. Pushing `refs/heads/main` into
a repository whose `HEAD` points somewhere else (a fresh `git init` still says
`master`) succeeds, and then hands every cloner nothing. So:

> **When the mirror repository is created, its default branch must be `main`
> before the first push** — or the first thing every user gets is an empty
> checkout of a plugin that "does not work".

### The publish steps, in order

```sh
sha=$(git subtree split --prefix=clients/nvim)
git push git@github.com:wolffe-lang/wolf.nvim.git "$sha:refs/heads/main"
git tag  -f client-nvim-v0.1.0 "$sha"
git push git@github.com:wolffe-lang/wolf.nvim.git client-nvim-v0.1.0
```

`doc/tags` is **committed**, not regenerated at publish time, so `:h wolf.nvim`
works on a fresh install for a user whose plugin manager never runs `:helptags`.
`cargo xtask nvim-check` fails if it goes stale, which is the check that makes
committing it safe.

### OWED TO HUMAN — the mirror repository

- [ ] Create `wolffe-lang/wolf.nvim`, **public**, default branch `main` (see
      the finding above).
- [ ] Add a deploy key with write access, or a fine-grained PAT scoped to that
      one repository, as `secrets.NVIM_MIRROR_KEY`. Not a broad org token: this
      job pushes to exactly one repository.
- [ ] Put the "GENERATED — do not send PRs here, send them to `wolf-lsp`" banner
      at the top of the mirror's README before the first push, not after.
- [ ] Then the lspconfig and mason entries in [`UPSTREAM.md`](UPSTREAM.md)
      become submittable — both are gated on a public installable `wolf` first.

### What users will be told, once it exists

```lua
-- lazy.nvim
{ 'wolffe-lang/wolf.nvim' }

-- packer
use 'wolffe-lang/wolf.nvim'

-- built-in packages, no manager at all
-- git clone https://github.com/wolffe-lang/wolf.nvim \
--   ~/.local/share/nvim/site/pack/wolf/start/wolf.nvim
```

Until the mirror exists, `clients/nvim/README.md` §Installing is the true
instruction, and it installs from a checkout.

---

## Helix — a fragment, and no channel at all

There is nothing to publish. The artifact is
[`clients/helix/languages.toml`](../clients/helix/languages.toml), a fragment a
user appends to `~/.config/helix/languages.toml`; Helix has no plugin system and
no package registry, so "distribution" is a documentation problem entirely.

Two consequences, both deliberate:

- **The compatibility statement cannot be carried by the artifact.** A TOML
  fragment cannot run code, so there is no runtime version check and there never
  will be. [`COMPAT.md`](COMPAT.md) is the whole statement.
- **Upstreaming is possible and is not proposed.** Helix ships language support
  in-tree (`languages.toml` in `helix-editor/helix`), so wolf could become
  built-in. That PR should not be opened before a public `wolf` exists — an
  editor adding built-in support for a language nobody can install is asking for
  a revert — and it is recorded as a future row, not an open one.

Verification stays where it is: `cargo xtask helix-health` runs `hx --health` in
CI on three OSes. Note the trap that check exists for — `hx --health` **always
exits 0**, even for a language it has never heard of, so the gate asserts on its
output lines rather than its exit code.

---

## Zed — a wasm component, and a registry that expects a submodule

### The shape

A Zed extension supplying a language server is not config: `language_server_command`
is a Rust function compiled to a WebAssembly component. Two facts drive
everything about publishing it:

- **The target is `wasm32-wasip2`.** Zed's `extension_builder.rs` pins
  `const RUST_TARGET: &str = "wasm32-wasip2"`. ls06's brief says `wasip1` and is
  out of date; building for wasip1 produces a component Zed will not load. CI
  job `zed-extension` builds the real target, and that build is the entire T2
  claim.
- **`zed_extension_api` is pinned at 0.7.0**, the newest version published to
  crates.io. 0.8.0 exists in the Zed tree with `publish = false`. An extension
  built against an older API stays compatible with newer Zed; the incompatible
  direction is the other one.

### The channels

| channel | mechanism | state |
|---|---|---|
| dev extension | `zed: install dev extension` from the command palette, pointed at `clients/zed/` | **the only path today.** It is a GUI action: Zed's CLI has no `--install-extension` and no `--dev-extension` flag, and `auto_install_extensions` in `settings.json` matches published extensions by id, not dev extensions |
| registry | a PR to `zed-industries/extensions` adding this extension as a **git submodule** plus a row in `extensions.toml`; Zed's infrastructure builds the wasm | `NOT SUBMITTED` ([`UPSTREAM.md`](UPSTREAM.md)) |

The registry PR requires a **public repository** for the submodule to point at,
and it requires the version in `extension.toml` to be bumped on every
resubmission — the registry rejects a re-publish of a version it already has.

### The gate that is not about credentials

Zed has **never been run against this extension**, by CI or by a human
(`MATRIX.md`). No machine this repository has run on has had Zed installed. So
the ordering is: install it as a dev extension, record a session, derive
`profiles/zed.json`, stamp the matrix row — *then* consider the registry.
Submitting an extension nobody has ever loaded is precisely the fabrication
`MATRIX.md` exists to prevent.

A user who installs it today gets a working language server and **no syntax
highlighting**: `[grammars.wolf]` ships commented out because Zed builds every
grammar named in the manifest *at install time*, and pointing it at the empty
`wolffe-lang/tree-sitter-wolf` would fail the install and take the language
server down with it.

---

## Emacs — manual, and why not MELPA

The artifact is one file,
[`clients/emacs/wolf-mode.el`](../clients/emacs/wolf-mode.el), quoted verbatim in
its README so it can be **pasted** rather than installed. That is the shipping
channel today and it works with no infrastructure at all.

**MELPA is explicitly a non-target at v1** (ls07). What it would require, so the
decision can be revisited on evidence rather than re-litigated:

| requirement | state |
|---|---|
| a **public** git repository with the `.el` at a stable path | wolf-lsp is private |
| a recipe PR to `melpa/melpa`: `(wolf-mode :fetcher github :repo "…" :files ("clients/emacs/*.el"))` | not submitted; MELPA does accept a `:files` subdirectory, so no mirror is needed here |
| package headers MELPA lints | **partly present.** The summary line with `-*- lexical-binding: t; -*-`, the `Commentary` section, `(provide 'wolf-mode)` and the `;;; wolf-mode.el ends here` footer are all there; `;; Version:`, `;; Package-Requires: ((emacs "29.1"))` and `;; URL:` are **not**, and the file's own Commentary says outright that it is not a package |
| a maintainer answering MELPA review and breakage | nobody has volunteered |

Adding the three missing headers is not a one-line change either: a test asserts
that `clients/emacs/README.md` contains `wolf-mode.el` **byte for byte**, so the
snippet a reader pastes and the file CI executes cannot drift. Header lines that
exist only to satisfy a package archive would go into the paste too. That is a
fair trade for a real MELPA listing and a bad one for a hypothetical, which is
why the headers are absent rather than speculative.

The floor is **Emacs 29**, because eglot has been bundled since 29 and that is
what makes the LSP half a zero-dependency snippet. `lsp-mode` users get a
three-line `lsp-register-client` in the README and no shipped artifact; nothing
in this repository has ever run `lsp-mode` (`MATRIX.md` T3 row).

Like Helix, this client carries **no runtime version check**, and for a reason
worth stating: a snippet a user pastes into `init.el` that then compares
versions against a declared range is a snippet nobody pastes.

---

## JetBrains — documented, and staying that way

No plugin, no JetBrains Marketplace listing, no paid-marketplace publishing.
`clients/jetbrains/README.md` is a recipe for LSP4IJ and it carries an honest
`NEVER` stamp: nobody has walked it end to end, because no JetBrains IDE is
installed anywhere this repository runs. That is a T3 row (`MATRIX.md`), and T3
means docs only.

---

## fackr and facsimile — distribution is somebody else's repository

Tier 0 clients are not artifacts this repository cuts. Their deliverable is a
patch series applied to somebody else's editor, so the compatibility statement
travels *with the patch*, into their repository, under their version scheme —
which is exactly why neither carries a `compat.json` and `cargo xtask
compat-check` fails if one appears.

Status of every patch: [`UPSTREAM.md`](UPSTREAM.md).

---

## What is never distributed from here, at any point

- **No `wolf` binary. No bundled server, no auto-download, no toolchain
  manager, no homebrew tap, no AUR package, no `.deb`.** D34 gives the compiler
  exactly one publisher, and it is not this repository. Every client resolves a
  `wolf` the user already has, and says so plainly when there is none.
- **No telemetry, no analytics, no update check.** The only network-shaped thing
  any client does is run `wolf --version` locally.
- **No paid-marketplace or JetBrains plugin publishing**, and no MELPA recipe at
  v1 — all three are ls07 non-targets.
