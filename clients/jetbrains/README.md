# jetbrains

**Tier 3 — the documented tier. No plugin is authored here, and none is
planned for v1.** This directory contains a recipe and nothing else: there is no
`build.gradle.kts`, no `plugin.xml`, and no artifact to install. If you came
looking for "the wolf plugin for IntelliJ", it does not exist.

That is a decision rather than a backlog item, and the reason is structural.

## Why there is no plugin

JetBrains ships two ways to speak LSP, and neither is a good foundation for a
v1 open-source plugin.

**The vendor LSP API is commercial-IDE-gated.** `com.intellij.platform.lsp`
landed in 2023.2 and is available only in the paid IDEs — IntelliJ IDEA
Ultimate, PyCharm Professional, WebStorm, GoLand, RustRover, CLion and the rest
— and explicitly **not** in IntelliJ IDEA Community or PyCharm Community. A
plugin built on it cannot run for a reader who installed the free IDE, which is
the reader a language's first-year documentation has to serve.

**LSP4IJ is a third-party plugin, and it works everywhere.** It is
open-source, it runs on Community editions, and it registers a language server
from a settings dialog with no code at all. Since it needs no plugin from us,
authoring one would buy nothing except a second thing to version.

So the recommendation is LSP4IJ, and the maintenance cost of this row is one
page of prose.

## Setup — LSP4IJ (recommended)

1. Install the **LSP4IJ** plugin: *Settings → Plugins → Marketplace*, search
   `LSP4IJ`.
2. Register `.lu` as a file type. LSP4IJ can only attach a server to a file type
   the IDE knows: *Settings → Editor → File Types → New…*, name it `Wolf`, and
   add the pattern `*.lu`. Set the line comment to `//` and leave the block
   comment fields **empty** — wolf has no block comment form, and filling them
   in gives *Comment with Block Comment* a construct the lexer rejects.
3. Add the server: *Settings → Languages & Frameworks → Language Servers → +*
   - **Name**: `wolf`
   - **Command**: `wolf lsp`
   - **Mappings → File type**: `Wolf`
4. Open a `.lu` file.

`wolf lsp` **is** the compiler (D34), so there is no server to install
separately and no version to keep in sync with anything. If `wolf` is not on the
IDE's `PATH` — and a GUI-launched IDE on macOS frequently has a different `PATH`
than your shell — use an absolute path in the **Command** field.

## Setup — the vendor LSP API (paid IDEs only)

If you are on a commercial IDE and prefer the built-in client, it still needs a
plugin to be written; there is no settings-only path. That plugin is roughly a
`LspServerSupportProvider` that starts `wolf lsp` for `*.lu`, plus a file-type
registration. **This repository does not ship one**, and this section exists so
that the option is known rather than so that it is followed.

## What works

Diagnostics, hover, go-to-definition where the server serves it, document
symbols, formatting and code actions — everything `wolf lsp` advertises, routed
through LSP4IJ's generic UI.

## What does NOT work, explicitly

- **No wolf-specific inspections.** IntelliJ's inspection framework runs on its
  own PSI, not on LSP diagnostics. Wolf's diagnostics arrive as LSP annotations;
  they do not appear in *Analyze → Inspect Code*, do not participate in
  inspection profiles, and cannot be suppressed per-inspection.
- **No refactoring integration.** *Rename* (`Shift+F6`), *Extract*, *Change
  Signature* and the rest are PSI operations. LSP `textDocument/rename` is not
  wired into them; only what LSP4IJ surfaces is available.
- **No structure view, no navigation bar, no *Find Usages* integration**, beyond
  what LSP4IJ synthesises from `documentSymbol`.
- **No syntax highlighting.** A file type registered by hand gets brace matching
  and comment folding, not tokens. There is no TextMate bundle for wolf in this
  repository (`clients/vscode/syntaxes/` is TextMate but is
  VS Code-shaped: `.tmLanguage.json` with a VS Code `package.json`
  contribution). Importing it into IntelliJ's TextMate bundle support is
  plausible and **has not been tried**, so it is not documented as a step.
- **No build/run integration**, no *Run Configuration* for `wolf`, and no
  problem-matcher equivalent.

## Verification

**T3 means a human, once per release, on a clean machine** — there is no CI lane
here and there will not be one: driving a JetBrains IDE headlessly to assert a
settings dialog was filled in correctly costs more to maintain than the row is
worth.

The row in [`docs/MATRIX.md`](../../docs/MATRIX.md) carries the wolf version and
the date it was last walked through. **A row whose stamp predates the current
release renders as unverified**, so this page cannot quietly rot into a claim.

At the time of writing this page has **never been walked end-to-end** — no
JetBrains IDE is installed on any machine this repository has run on — and the
matrix row says exactly that rather than carrying a date nobody earned.
