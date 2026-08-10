// The publish DRY RUN (ls07 §1), and the reason it has to be hand-built.
//
// ============================================================================
// NOTHING IN THIS FILE CONTACTS A MARKETPLACE, AND NOTHING PUBLISHES.
// ============================================================================
//
// `@vscode/vsce` has **no `--dry-run` flag**. `vsce publish` either uploads or
// it errors; the closest thing to a rehearsal is `vsce verify-pat`, which needs
// a real Personal Access Token for a real publisher, and this extension's
// publisher (`wolf-lang-unpublished`) is a deliberate placeholder that is not
// registered anywhere. So the rehearsal is assembled out of the three things
// that CAN run offline, and together they cover everything `vsce publish` would
// check before it opened a socket:
//
//   1. `vsce package` — the same manifest validation, the same prepublish
//      script, the same file selection, ending in a real vsix. This runs in the
//      `vscode-extension` CI job, and its output is what the artifact upload
//      keeps per run.
//   2. `vsce ls` — the exact file list the upload would carry. Compared here
//      against a REVIEWED list (`PACKAGE-CONTENTS.txt`), so a `.vscodeignore`
//      change or a new runtime dependency is a diff somebody reads rather than
//      a surprise in a user's download.
//   3. The manifest lint below — the marketplace-listing fields `vsce` does not
//      enforce but a listing is judged by, split into what is CHECKED and what
//      is OWED TO A HUMAN.
//
// What remains after all three is exactly one thing: the upload. That is the
// gate, it is documented in `docs/DISTRIBUTION.md`, and it cannot be crossed
// from this repository because no publisher identity and no token exist.

import { execFileSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

const EXT_ROOT = path.resolve(__dirname, '..', '..');
const REPO_ROOT = path.resolve(EXT_ROOT, '..', '..');
const CONTENTS = path.join(EXT_ROOT, 'PACKAGE-CONTENTS.txt');

/// Written into the generated list so the next reader knows it is reviewed
/// output rather than a lock file. `//` lines are skipped when it is read back.
const HEADER = [
	'// REVIEWED: what `vsce package` puts in the vsix, projected.',
	'//',
	'// Regenerate with `UPDATE_SNAPSHOTS=1 npm run test:manifest` and then READ THE',
	'// DIFF — that reading is the entire purpose of this file. A line appearing here',
	'// is a decision to ship a file to every user; a line disappearing is a decision',
	'// to stop. `npm run test:manifest` fails on any difference.',
	'//',
	'// The interior of `node_modules` is collapsed to the package SET on purpose:',
	'// the raw `vsce ls` output is ~200 lines of a dependency nobody here controls,',
	'// and a reviewer who has to skim those stops reading the ten that matter.',
	'',
].join('\n');

let failures = 0;
let pending = 0;

function ok(what: string): void {
	console.log(`ok       ${what}`);
}

function fail(what: string, detail: string): void {
	failures += 1;
	console.log(`FAIL     ${what}\n         ${detail}`);
}

function owed(what: string, human: string): void {
	pending += 1;
	console.log(`PENDING  ${what}\n         HUMAN: ${human}`);
}

// ------------------------------------------------- the packaged contents --

/// The projection of `vsce ls` that is worth reviewing.
///
/// Not the raw 211-line list: two hundred of those lines are the interior of
/// `vscode-languageclient`, they churn on every patch release of a dependency
/// nobody here controls, and a reviewer who has to skim them stops reading the
/// nine that matter. So the reviewed artifact is the files THIS repository
/// authors, plus the SET of runtime dependency packages — which is the pair
/// that actually answers "what did we just decide to ship".
function projection(): string[] {
	const raw = execFileSync(
		path.join(EXT_ROOT, 'node_modules', '.bin', process.platform === 'win32' ? 'vsce.cmd' : 'vsce'),
		['ls'],
		{ cwd: EXT_ROOT, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 },
	);
	const authored: string[] = [];
	const deps = new Set<string>();
	for (const line of raw.split(/\r?\n/)) {
		const file = line.trim();
		if (file === '') {
			continue;
		}
		if (file.startsWith('node_modules/')) {
			// `node_modules/<pkg>/…` and `node_modules/<a>/node_modules/<b>/…`;
			// the nested form is a hoisting detail, and the claim being
			// reviewed is which packages ship at all.
			const parts = file.split('/');
			for (let i = 0; i < parts.length - 1; i++) {
				if (parts[i] === 'node_modules') {
					deps.add(parts[i + 1]!);
				}
			}
			continue;
		}
		authored.push(file);
	}
	authored.sort();
	return [
		'# authored files',
		...authored,
		'# runtime dependency packages',
		...[...deps].sort().map((d) => `node_modules/${d}`),
	];
}

function checkContents(): void {
	const got = projection();

	// Before the read, not after: the review artifact has to be creatable from
	// nothing, and a read-then-update order makes `UPDATE_SNAPSHOTS=1` work for
	// every case except the first one — which is the case somebody bootstrapping
	// the list is in.
	if (process.env.UPDATE_SNAPSHOTS === '1') {
		fs.writeFileSync(CONTENTS, `${HEADER}${got.join('\n')}\n`);
		console.log(`wrote    ${CONTENTS} — now READ the diff`);
		return;
	}

	let want: string[];
	try {
		want = fs
			.readFileSync(CONTENTS, 'utf8')
			.split(/\r?\n/)
			.filter((l) => l.trim() !== '' && !l.startsWith('//'));
	} catch {
		fail(
			'the packaged contents match the reviewed list',
			`${CONTENTS} is missing — create it with \`UPDATE_SNAPSHOTS=1 npm run test:manifest\`, ` +
				'then read what it wrote',
		);
		return;
	}
	const missing = want.filter((l) => !got.includes(l));
	const extra = got.filter((l) => !want.includes(l));
	if (missing.length === 0 && extra.length === 0) {
		ok(`the vsix ships exactly the reviewed ${got.length - 2} entries`);
		return;
	}
	fail(
		'the packaged contents match the reviewed list',
		`no longer shipped: ${missing.join(', ') || '(none)'}\n         ` +
			`newly shipped:   ${extra.join(', ') || '(none)'}\n         ` +
			'review, then `UPDATE_SNAPSHOTS=1 npm run test:manifest`',
	);
}

// --------------------------------------------------------- the manifest --

function checkManifest(): void {
	const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));

	// Fields the marketplace requires, or that a publish would reject on.
	for (const field of ['name', 'displayName', 'description', 'version', 'publisher', 'engines', 'license', 'categories', 'repository']) {
		if (pkg[field] === undefined) {
			fail('the manifest carries every field a publish requires', `no \`${field}\``);
			return;
		}
	}
	ok('the manifest carries every field a publish requires');

	// `vsce` rejects `*` activation without `--allow-star-activation`, and a
	// star-activating language extension is a slow window for everybody.
	if ((pkg.activationEvents ?? []).includes('*')) {
		fail('activation is scoped', '`*` activation would need --allow-star-activation');
	} else {
		ok(`activation is scoped (${(pkg.activationEvents ?? []).join(', ')})`);
	}

	// The license file the packager warns about, and the drift check that makes
	// the copy safe.
	const licence = path.join(EXT_ROOT, 'LICENSE.md');
	if (!fs.existsSync(licence)) {
		fail('a license ships inside the vsix', 'clients/vscode/LICENSE.md is missing');
	} else {
		const text = fs.readFileSync(licence, 'utf8');
		const drifted = ['LICENSE-MIT', 'LICENSE-APACHE'].filter(
			(f) => !text.includes(fs.readFileSync(path.join(REPO_ROOT, f), 'utf8').trimEnd()),
		);
		if (drifted.length > 0) {
			fail('a license ships inside the vsix', `LICENSE.md no longer quotes ${drifted.join(', ')} verbatim`);
		} else {
			ok('a license ships inside the vsix, quoting both root license files verbatim');
		}
	}

	// The compatibility statement travels WITH the artifact (ls07 §3: "in
	// machine-readable form"), so a user who has only the vsix can still answer
	// "which wolf is this for". `.vscodeignore` must not learn to drop it.
	if (!fs.existsSync(path.join(EXT_ROOT, 'compat.json'))) {
		fail('the compatibility statement ships', 'clients/vscode/compat.json is missing');
	} else {
		ok('the compatibility statement ships inside the vsix (compat.json)');
	}

	// --- the placeholder, stated as the blocker it is --------------------
	if (String(pkg.publisher).includes('unpublished')) {
		owed(
			`the publisher identity (\`${pkg.publisher}\` is a placeholder)`,
			'register a VS Marketplace publisher and an Open VSX namespace, then replace this ' +
				'field IN THE SAME COMMIT that first publishes. docs/DISTRIBUTION.md §"OWED TO HUMAN".',
		);
	} else {
		ok(`publisher is \`${pkg.publisher}\``);
	}

	// Listing content the sprint names as a deliverable and that no program can
	// produce: art, and a screenshot of a real editor showing real diagnostics.
	if (pkg.icon === undefined) {
		owed(
			'a marketplace icon',
			'draw or commission a 128x128 PNG, add `"icon": "images/icon.png"`. Not generated ' +
				'here: inventing branding art is a design decision, not a build step.',
		);
	} else {
		ok(`icon: ${pkg.icon}`);
	}
	if ((pkg.keywords ?? []).length === 0) {
		owed(
			'marketplace keywords',
			'decide the search terms this listing should win. An empty keyword list is a ' +
				'listing nobody finds, and guessing them here would be guessing at marketing.',
		);
	} else {
		ok(`keywords: ${(pkg.keywords ?? []).join(', ')}`);
	}
	owed(
		'a screenshot of real diagnostics from a real corpus sample, above the fold in README.md',
		'ls07 §1 requires a screenshot and forbids a mockup. Take it against a live `wolf lsp` ' +
			'on a vendored sample once a wolf release exists (step 0 of docs/RELEASE.md).',
	);
}

// ---------------------------------------------------------------- main --

console.log('vsce publish DRY RUN — this command publishes nothing and cannot.\n');
checkContents();
checkManifest();
console.log(
	`\n${failures} failed, ${pending} owed to a human.\n` +
		'The only step left after this is the upload itself, which needs a registered\n' +
		'publisher and a token. See docs/DISTRIBUTION.md.',
);
if (failures > 0) {
	process.exit(1);
}
