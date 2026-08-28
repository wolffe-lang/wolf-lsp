// The grammar lane: does `syntaxes/wolf.tmLanguage.json` tokenize the pinned
// corpus the way we say it does?
//
// This lane needs **no VS Code and no `wolf` binary**, which is why it is a
// separate entry point rather than a case inside the extension suite. It uses
// `vscode-textmate` and `vscode-oniguruma` — the exact tokenizer and the exact
// regex engine VS Code itself runs — so a snapshot taken here is a real claim
// about what a user's editor will render, not an approximation of one.
//
// The snapshots under `snapshots/` are reviewed like any other snapshot in this
// repository (CONTRIBUTING.md, "the snapshot ritual"): regenerate with
// `UPDATE_SNAPSHOTS=1 npm run test:grammar`, then READ the diff. Two things
// they are deliberately good at showing:
//
//   * the four string forms resolving in the right order, with `{…}`
//     interpolation opening an embedded scope inside all of them, and
//   * the operator gap. `strings.lu` contains `==` and `&&`, and the snapshot
//     records them as unscoped, because the pinned EBNF does not carry the
//     precedence climb. That is the gap `clients/vscode/inventory.md` states,
//     printed rather than described — and the day upstream vendors §3.2, this
//     snapshot changes and someone has to look at it.
//
// No `.lu` file is authored here. The subjects are vendored corpus samples, for
// the reason `tests/nvim_plugin.rs` asserts for the whole of `clients/`.

import * as fs from 'fs';
import * as path from 'path';

import * as oniguruma from 'vscode-oniguruma';
import * as vsctm from 'vscode-textmate';

/// `clients/vscode/` — this file compiles to `out/test/grammar.js`.
const EXT_ROOT = path.resolve(__dirname, '..', '..');
const REPO_ROOT = path.resolve(EXT_ROOT, '..', '..');
const SAMPLES = path.join(REPO_ROOT, 'vendor', 'upstream', 'samples');
const SNAPSHOTS = path.join(EXT_ROOT, 'src', 'test', 'snapshots');

/// The subjects, and why each one.
///
/// Two, not ten. A snapshot nobody rereads is a snapshot nobody reviews, and
/// these two between them reach every rule in the grammar's repository:
const SUBJECTS: ReadonlyArray<readonly [string, string]> = [
	['hello.lu', 'doc comments, keywords, the plain string form, `{who}` interpolation'],
	[
		'strings.lu',
		'every string form — `"""` block, `re"…"` generalized, plain — plus a ' +
			'`{words:>3}` format spec, `^13` from-end indexing, and `==`/`&&` ' +
			'coloured as single operators now that the pinned EBNF renders the ' +
			'full precedence climb (f9ee9aa)',
	],
];

// ------------------------------------------------------------ tokenizing --

async function registry(): Promise<vsctm.Registry> {
	const wasm = fs.readFileSync(require.resolve('vscode-oniguruma/release/onig.wasm'));
	await oniguruma.loadWASM(
		wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength) as ArrayBuffer,
	);

	return new vsctm.Registry({
		onigLib: Promise.resolve({
			createOnigScanner: (sources: string[]) => new oniguruma.OnigScanner(sources),
			createOnigString: (s: string) => new oniguruma.OnigString(s),
		}),
		loadGrammar: async (scopeName: string) => {
			const file: Record<string, string> = {
				'source.wolf': 'wolf.tmLanguage.json',
				'source.wolfi': 'wolfi.tmLanguage.json',
				'source.wolf-pkg': 'wolf-pkg.tmLanguage.json',
			};
			const name = file[scopeName];
			if (name === undefined) {
				return null;
			}
			const full = path.join(EXT_ROOT, 'syntaxes', name);
			return vsctm.parseRawGrammar(fs.readFileSync(full, 'utf8'), full);
		},
	});
}

/// One reviewable line per non-whitespace token.
///
/// The leading `source.wolf` is stripped from every stack — it is on every
/// token and carries no information — so a token with nothing left prints `—`.
/// Those dashes are the point: they are exactly what the grammar does not
/// colour.
function snapshot(grammar: vsctm.IGrammar, text: string): string {
	const out: string[] = [];
	let stack = vsctm.INITIAL;
	const lines = text.split('\n');
	for (let i = 0; i < lines.length; i++) {
		const line = lines[i]!;
		const result = grammar.tokenizeLine(line, stack);
		for (const token of result.tokens) {
			const piece = line.substring(token.startIndex, token.endIndex);
			if (piece.trim() === '') {
				continue;
			}
			const scopes = token.scopes.filter((s) => s !== 'source.wolf');
			out.push(
				`${String(i + 1).padStart(3)} ${JSON.stringify(piece).padEnd(28)} ` +
					`${scopes.length === 0 ? '—' : scopes.join(' ')}`,
			);
		}
		stack = result.ruleStack;
	}
	return out.join('\n') + '\n';
}

// ----------------------------------------------------------------- cases --

let passed = 0;
const failures: string[] = [];

function check(name: string, fn: () => void): void {
	try {
		fn();
		passed += 1;
		console.log(`ok    ${name}`);
	} catch (e) {
		failures.push(name);
		console.log(`FAIL  ${name}\n${e instanceof Error ? e.stack : String(e)}`);
	}
}

async function main(): Promise<void> {
	const reg = await registry();
	const grammar = await reg.loadGrammar('source.wolf');
	if (grammar === null) {
		throw new Error('source.wolf did not load — is syntaxes/wolf.tmLanguage.json valid?');
	}

	const update = process.env['UPDATE_SNAPSHOTS'] === '1';

	for (const [sample, why] of SUBJECTS) {
		check(`${sample} tokenizes as reviewed (${why})`, () => {
			const text = fs.readFileSync(path.join(SAMPLES, sample), 'utf8');
			const got = snapshot(grammar, text);
			const file = path.join(SNAPSHOTS, `${sample.replace(/\//g, '-')}.txt`);
			if (update) {
				fs.mkdirSync(SNAPSHOTS, { recursive: true });
				fs.writeFileSync(file, got);
				return;
			}
			const want = fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : '';
			if (got !== want) {
				const g = got.split('\n');
				const w = want.split('\n');
				const at = g.findIndex((l, i) => l !== w[i]);
				throw new Error(
					`scope snapshot drift in ${file}\n` +
						`  first difference at snapshot line ${at + 1}\n` +
						`  committed: ${w[at] ?? '<end of file>'}\n` +
						`  produced : ${g[at] ?? '<end of file>'}\n` +
						`Review it, then re-record with UPDATE_SNAPSHOTS=1 npm run test:grammar`,
				);
			}
		});
	}

	// The two include-only grammars are not decoration: if either stops
	// resolving `source.wolf`, `.wolfi` and `wolf.pkg` silently lose ALL
	// highlighting, and nothing else in this repository would notice.
	//
	// Loaded before the assertion rather than inside it, because `check` takes
	// a synchronous closure — an `async` one would resolve to a promise the
	// runner never awaits, and every such case would "pass" without running.
	for (const scope of ['source.wolfi', 'source.wolf-pkg']) {
		const included = await reg.loadGrammar(scope);
		check(`${scope} resolves through to source.wolf`, () => {
			if (included === null) {
				throw new Error(`${scope} did not load at all`);
			}
			const tokens = included.tokenizeLine('fn main() -> !int {', vsctm.INITIAL).tokens;
			const fn = tokens.find((t) => t.scopes.includes('storage.type.wolf'));
			if (fn === undefined) {
				throw new Error(
					`${scope} tokenized \`fn\` without source.wolf's storage.type.wolf — ` +
						`the include is broken, and this language just lost ALL highlighting`,
				);
			}
		});
	}

	console.log(`\n${passed} passed, ${failures.length} failed`);
	for (const f of failures) {
		console.log(`  failed: ${f}`);
	}
	process.exit(failures.length === 0 ? 0 : 1);
}

main().catch((e) => {
	console.error(e);
	process.exit(1);
});
