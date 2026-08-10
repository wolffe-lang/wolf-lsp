// Launch a real, headless VS Code and run `suite/` inside it.
//
// The workspace is the vendored corpus (`vendor/upstream/samples`), for the
// same reason every other lane in this repository uses it: those bytes are
// canonical `wolf fmt` output at a STYLE_VERSION, so "formatting produced no
// edits" is a real claim rather than a tautology about a file we wrote.
//
// `WOLF_BIN`, when set, is put on `PATH` **as a directory** rather than passed
// through `wolf.serverPath`. That is deliberate: it exercises the discovery
// path a user actually has (a toolchain on `PATH`, no setting at all), and it
// is the same mechanism the capture proxy in `README.md` relies on.

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';

import { runTests } from '@vscode/test-electron';

async function main(): Promise<void> {
	const extensionDevelopmentPath = path.resolve(__dirname, '..', '..');
	const extensionTestsPath = path.resolve(__dirname, 'suite', 'index');
	const repoRoot = path.resolve(extensionDevelopmentPath, '..', '..');
	const workspace = path.join(repoRoot, 'vendor', 'upstream', 'samples');

	// A throwaway profile. Without it a test run reads — and writes — the
	// developer's real VS Code settings, which is both a correctness problem
	// (their `editor.formatOnSave` becomes part of the test) and a rude one.
	const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'wolf-vscode-test-'));

	const extensionTestsEnv: Record<string, string | undefined> = {
		WOLF_LSP_REPO_ROOT: repoRoot,
	};
	const wolfBin = process.env['WOLF_BIN'];
	if (wolfBin !== undefined && wolfBin !== '') {
		extensionTestsEnv['PATH'] =
			path.dirname(wolfBin) + path.delimiter + (process.env['PATH'] ?? '');
	}

	const launchArgs = [
		workspace,
		// Other extensions stay out of it; the one under development still
		// loads. A machine-dependent extension set is a machine-dependent
		// test result.
		'--disable-extensions',
		'--disable-workspace-trust',
		`--user-data-dir=${userDataDir}`,
	];
	if (process.platform === 'linux') {
		// Containers without a sandbox-capable kernel config, which is most
		// CI images. Harmless where the sandbox would have worked.
		launchArgs.push('--no-sandbox', '--disable-gpu');
	}

	try {
		await runTests({ extensionDevelopmentPath, extensionTestsPath, launchArgs, extensionTestsEnv });
	} finally {
		fs.rmSync(userDataDir, { recursive: true, force: true });
	}
}

main().catch((err) => {
	console.error('the headless VS Code lane failed');
	console.error(err);
	process.exit(1);
});
