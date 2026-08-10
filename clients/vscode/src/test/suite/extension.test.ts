// The extension lane, inside a real VS Code.
//
// Two halves, split by what they need — the same split
// `tests/nvim_plugin.rs` makes, and for the same reason:
//
//   * the ARTIFACT half needs no `wolf` binary. Contributions, activation,
//     language registration, the settings surface and the command set are all
//     true of a machine with no toolchain, which is the state every CI runner
//     this repository has is in. That half is what keeps this lane from being
//     dark.
//   * the SERVER half needs a `wolf` at the pin and **skips loudly** without
//     one (ls00 §3). A missing toolchain must not turn the suite red on a
//     machine that never asked for one, and must not pass silently either.
//
// Nothing here asserts diagnostic PROSE. D22 puts wording upstream under review
// there, and an editor-side test that pins a sentence makes this repository a
// second approval gate on it. Codes and positions are behaviour and are
// asserted exactly.

import { execFileSync } from 'child_process';
import * as path from 'path';

import * as assert from 'assert';
import * as vscode from 'vscode';

import { PIN } from '../../pin';

const EXTENSION_ID = 'wolf-lang-unpublished.wolf';

function samplesRoot(): string {
	const folder = vscode.workspace.workspaceFolders?.[0];
	assert.ok(folder, 'the test window opened with no workspace folder');
	return folder.uri.fsPath;
}

function sample(rel: string): vscode.Uri {
	return vscode.Uri.file(path.join(samplesRoot(), ...rel.split('/')));
}

async function open(rel: string): Promise<vscode.TextDocument> {
	const doc = await vscode.workspace.openTextDocument(sample(rel));
	await vscode.window.showTextDocument(doc, { preview: false });
	return doc;
}

/// Poll until `predicate` holds, or fail saying what was being waited for.
async function waitFor(what: string, predicate: () => boolean, ms = 20_000): Promise<void> {
	const deadline = Date.now() + ms;
	while (Date.now() < deadline) {
		if (predicate()) {
			return;
		}
		await new Promise((r) => setTimeout(r, 50));
	}
	assert.fail(`timed out after ${ms}ms waiting for: ${what}`);
}

/// The `wolf` binary at the pin, or the reason there is none.
///
/// Version equality with `PIN`, not mere presence: a stale local `wolf`
/// producing green results is the exact failure the pin exists to prevent
/// (`vendor/upstream/PIN`).
function server(): { ok: true; version: string } | { ok: false; reason: string } {
	let version: string;
	try {
		version = execFileSync('wolf', ['--version'], { encoding: 'utf8', timeout: 5000 }).trim();
	} catch {
		return { ok: false, reason: 'no `wolf` on PATH in the test window' };
	}
	if (version !== PIN.version) {
		return {
			ok: false,
			reason: `\`wolf --version\` is ${JSON.stringify(version)}, the pin expects ${JSON.stringify(PIN.version)}`,
		};
	}
	return { ok: true, version };
}

// =================================================== the artifact half ====

suite('wolf: contributions and activation (no toolchain needed)', () => {
	test('the extension activates when a `.lu` document opens', async () => {
		const ext = vscode.extensions.getExtension(EXTENSION_ID);
		assert.ok(ext, `no extension ${EXTENSION_ID} — did the publisher or name change?`);
		await open('hello.lu');
		await waitFor('the extension to activate', () => ext.isActive);
	});

	test('`.lu` is the `wolf` language', async () => {
		const doc = await open('hello.lu');
		assert.strictEqual(doc.languageId, 'wolf');
	});

	test('`.wolfi` is its own language, and deliberately not `wolf`', () => {
		// The narrowing this extension makes on purpose. `wolf lsp` has no
		// `.wolfi` path at this pin — the format is BINARY (`WOLFI` magic,
		// `upstream/crates/wolf_sema/src/interface.rs`) — so `.wolfi` gets a
		// language and highlighting, and no client attaches to it. A separate
		// id is what makes `documentSelector: [{ language: 'wolf' }]` express
		// that structurally instead of by convention.
		const ext = vscode.extensions.getExtension(EXTENSION_ID);
		assert.ok(ext);
		const languages = ext.packageJSON.contributes.languages as Array<{
			id: string;
			extensions?: string[];
			filenames?: string[];
		}>;
		const byExt = (e: string) => languages.find((l) => l.extensions?.includes(e))?.id;
		assert.strictEqual(byExt('.lu'), 'wolf');
		assert.strictEqual(byExt('.wolfi'), 'wolfi');
		const pkg = languages.find((l) => l.filenames?.includes('wolf.pkg'));
		assert.strictEqual(pkg?.id, 'wolf-pkg');
		assert.ok(pkg.filenames?.includes('wolf.sum'), 'wolf.sum shares the manifest language');
	});

	test('the three documented commands are registered', async () => {
		const all = await vscode.commands.getCommands(true);
		for (const id of ['wolf.restartServer', 'wolf.showServerLog', 'wolf.showVersion']) {
			assert.ok(all.includes(id), `command ${id} is not registered`);
		}
	});

	test('the settings surface is exactly the two the sprint names', () => {
		const ext = vscode.extensions.getExtension(EXTENSION_ID);
		assert.ok(ext);
		const props = Object.keys(ext.packageJSON.contributes.configuration.properties).sort();
		// `wolf.formatOnSave` is absent on purpose: VS Code already has
		// `editor.formatOnSave`, and a second switch for the same behaviour is
		// a bug report waiting to be filed. Anything beyond these two is a
		// compiler-track conversation (ls05 §1).
		assert.deepStrictEqual(props, ['wolf.serverPath', 'wolf.trace.server']);
		const config = vscode.workspace.getConfiguration('wolf');
		assert.strictEqual(config.get('serverPath'), '', 'default: `wolf` on PATH');
		assert.strictEqual(config.get('trace.server'), 'off');
	});

	test('every contributed grammar file exists', async () => {
		const ext = vscode.extensions.getExtension(EXTENSION_ID);
		assert.ok(ext);
		const grammars = ext.packageJSON.contributes.grammars as Array<{
			scopeName: string;
			path: string;
		}>;
		assert.strictEqual(grammars.length, 3);
		for (const g of grammars) {
			const uri = vscode.Uri.joinPath(ext.extensionUri, g.path);
			// Throws when absent — which is what a `.vscodeignore` that
			// excluded `syntaxes/` would produce, in a vsix nobody would
			// notice was broken until they opened a file.
			await vscode.workspace.fs.stat(uri);
		}
	});

	test('no UI is enabled for capabilities the server does not serve', () => {
		const ext = vscode.extensions.getExtension(EXTENSION_ID);
		assert.ok(ext);
		const contributes = JSON.stringify(ext.packageJSON.contributes);
		// s52 non-targets, restated as a test: semantic tokens and inlay hints
		// are post-v1 compiler work, and an extension that advertises them
		// produces an editor that looks broken rather than one that looks
		// early.
		for (const forbidden of ['semanticTokenScopes', 'semanticTokenTypes', 'inlayHint']) {
			assert.ok(
				!contributes.includes(forbidden),
				`package.json contributes \`${forbidden}\`, which the server does not serve`,
			);
		}
	});

	test('the language configuration matches the zero-option formatter', async () => {
		const doc = await open('hello.lu');
		const options = vscode.window.visibleTextEditors.find((e) => e.document === doc)?.options;
		assert.ok(options);
		// wolf_fmt: INDENT = 4, spaces. Read off the toolchain, not chosen.
		assert.strictEqual(options.insertSpaces, true);
		assert.strictEqual(options.tabSize, 4);
	});
});

// ==================================================== the server half ====

suite('wolf: the language server (needs `wolf` at the pin)', function () {
	const status = server();

	suiteSetup(function () {
		if (!status.ok) {
			// LOUD. A skipped server lane that says nothing is a dark lane
			// nobody notices is dark.
			console.log(`SKIP: the wolf server lane — ${status.reason}`);
			console.log('SKIP: diagnostics, hover, documentSymbol and formatting are NOT covered here.');
			this.skip();
		} else {
			console.log(`wolf: ${status.version} (pin ${PIN.commit.slice(0, 7)})`);
		}
	});

	test('a canonical sample publishes no diagnostics', async () => {
		const doc = await open('hello.lu');
		// Wait for the server to have spoken about the file at all, rather
		// than sampling an empty array before it started: `regions.lu` is
		// opened first as a barrier only if needed — here the clean publish
		// and "not yet answered" are indistinguishable, so the following
		// broken-file test is what proves the server is live.
		await new Promise((r) => setTimeout(r, 3000));
		assert.deepStrictEqual(vscode.languages.getDiagnostics(doc.uri), []);
	});

	test('a broken sample lands E0002 on the exact one-truth position', async () => {
		const doc = await open('grammar/semicolon.lu');
		await waitFor(
			'the server to publish a diagnostic for grammar/semicolon.lu',
			() => vscode.languages.getDiagnostics(doc.uri).length > 0,
		);
		const diagnostics = vscode.languages.getDiagnostics(doc.uri);
		const d = diagnostics[0]!;
		assert.strictEqual(String(typeof d.code === 'object' ? d.code.value : d.code), 'E0002');
		assert.strictEqual(d.severity, vscode.DiagnosticSeverity.Error);
		// `wolf conform-run grammar/semicolon.lu --error-format=json` puts this
		// span at bytes [222,223]: line 8, byte column 5 (1-based), so 0-based
		// line 7, character 4. The line is ASCII, so the utf-16 code-unit
		// column VS Code negotiated is the same number — this is the one-truth
		// claim made from inside the editor.
		assert.strictEqual(d.range.start.line, 7);
		assert.strictEqual(d.range.start.character, 4);
		// The message exists; its wording is upstream's (D22).
		assert.ok(d.message.length > 0);
	});

	test('hover answers with the type and an exact range', async () => {
		const doc = await open('hello.lu');
		const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
			'vscode.executeHoverProvider',
			doc.uri,
			new vscode.Position(9, 9),
		);
		assert.ok(hovers && hovers.length > 0, 'hover returned nothing for `who`');
		const text = hovers
			.flatMap((h) => h.contents)
			.map((c) => (typeof c === 'string' ? c : c.value))
			.join('\n');
		assert.ok(text.includes('who'), `hover names the binding: ${text}`);
		assert.ok(text.includes('str'), `hover names the type: ${text}`);
		const range = hovers.find((h) => h.range)?.range;
		assert.ok(range, 'hover carried no range');
		assert.strictEqual(range.start.character, 8);
		assert.strictEqual(range.end.character, 11);
	});

	test('documentSymbol returns the program’s one function', async () => {
		const doc = await open('hello.lu');
		const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
			'vscode.executeDocumentSymbolProvider',
			doc.uri,
		);
		assert.ok(symbols && symbols.length > 0, 'documentSymbol returned nothing');
		const names = symbols.map((s) => s.name);
		assert.ok(names.includes('main'), `expected \`main\` among ${JSON.stringify(names)}`);
	});

	test('formatting canonical bytes round-trips to no edits', async () => {
		const doc = await open('hello.lu');
		const edits = await vscode.commands.executeCommand<vscode.TextEdit[]>(
			'vscode.executeFormatDocumentProvider',
			doc.uri,
			{ tabSize: 4, insertSpaces: true },
		);
		// Corpus bytes ARE canonical `wolf fmt` output at STYLE_VERSION 1, so
		// an edit here would mean the formatter disagrees with the corpus — a
		// real finding, not a test to relax.
		assert.deepStrictEqual(edits ?? [], [], 'formatting rewrote a canonical file');
	});

	test('a code action arrives fully resolved for the one machine-applicable fix', async () => {
		const doc = await open('grammar/semicolon.lu');
		await waitFor(
			'the diagnostic the code action hangs off',
			() => vscode.languages.getDiagnostics(doc.uri).length > 0,
		);
		const range = vscode.languages.getDiagnostics(doc.uri)[0]!.range;
		const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
			'vscode.executeCodeActionProvider',
			doc.uri,
			range,
		);
		assert.ok(actions && actions.length > 0, 'no code action for E0002');
		assert.ok(
			actions.some((a) => a.edit !== undefined),
			'wolf resolves its fix-its at publish time; an action with no edit means that changed',
		);
	});
});
