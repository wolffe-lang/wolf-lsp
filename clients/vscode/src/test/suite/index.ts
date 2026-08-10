// The mocha entry point VS Code calls once the window is up.
//
// `fs.readdirSync` rather than `glob`: one directory of `*.test.js` does not
// justify a dependency, and this repository's posture on that is settled
// (`Cargo.toml`'s workspace comment; `clients/nvim/README.md`'s "Why there is
// no busted or plenary").

import * as fs from 'fs';
import * as path from 'path';

import Mocha from 'mocha';

export function run(): Promise<void> {
	const mocha = new Mocha({
		ui: 'tdd',
		color: true,
		// A cold VS Code plus a compiler start plus a first parse. The
		// per-assertion waits inside the suite are much tighter than this;
		// this is only the outer bound that turns a hang into a failure.
		timeout: 60_000,
	});

	const testsRoot = __dirname;
	for (const file of fs.readdirSync(testsRoot).sort()) {
		if (file.endsWith('.test.js')) {
			mocha.addFile(path.join(testsRoot, file));
		}
	}

	return new Promise((resolve, reject) => {
		try {
			mocha.run((failures) => {
				if (failures > 0) {
					reject(new Error(`${failures} test(s) failed.`));
				} else {
					resolve();
				}
			});
		} catch (err) {
			reject(err instanceof Error ? err : new Error(String(err)));
		}
	});
}
