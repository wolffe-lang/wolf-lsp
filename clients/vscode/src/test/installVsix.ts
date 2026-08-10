// Install the packaged vsix into a throwaway profile, and fail if it does not
// take.
//
// This is the install path `README.md` documents, run as a test so the
// instructions cannot rot (ls05 §3: the vsix is a first-class documented path,
// not a developer footnote). It reuses the VS Code build `@vscode/test-electron`
// already downloaded for the extension lane rather than requiring a `code` on
// `PATH`, because on a CI runner there is no such thing — and on a developer's
// machine, using theirs would install into their real profile.
//
// `--extensions-dir` and `--user-data-dir` both point at temporary
// directories. Installing into the default profile would leak into whatever
// else the machine does, and an install nobody can repeat is not evidence.

import { spawnSync } from 'child_process';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';

import {
	downloadAndUnzipVSCode,
	resolveCliArgsFromVSCodeExecutablePath,
} from '@vscode/test-electron';

async function main(): Promise<void> {
	const extRoot = path.resolve(__dirname, '..', '..');
	const version = JSON.parse(
		fs.readFileSync(path.join(extRoot, 'package.json'), 'utf8'),
	).version as string;
	const vsix = path.join(extRoot, `wolf-${version}.vsix`);
	if (!fs.existsSync(vsix)) {
		throw new Error(`${vsix} does not exist — run \`npm run package\` first`);
	}

	const extensionsDir = fs.mkdtempSync(path.join(os.tmpdir(), 'wolf-vsix-ext-'));
	const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'wolf-vsix-ud-'));
	try {
		const exe = await downloadAndUnzipVSCode();
		const [cli, ...resolved] = resolveCliArgsFromVSCodeExecutablePath(exe);
		if (cli === undefined) {
			throw new Error('could not resolve the VS Code CLI path');
		}
		// `resolveCliArgsFromVSCodeExecutablePath` supplies its OWN
		// `--extensions-dir`/`--user-data-dir` pair. VS Code takes the last
		// occurrence and warns about the rest, so the duplicates are dropped
		// here rather than left to a precedence rule: an install whose target
		// directory depends on argument order is not the install anyone meant.
		const baseArgs: string[] = [];
		for (let i = 0; i < resolved.length; i++) {
			const arg = resolved[i]!;
			if (arg === '--extensions-dir' || arg === '--user-data-dir') {
				i += 1; // skip its value too
				continue;
			}
			if (arg.startsWith('--extensions-dir=') || arg.startsWith('--user-data-dir=')) {
				continue;
			}
			baseArgs.push(arg);
		}
		const profile = ['--extensions-dir', extensionsDir, '--user-data-dir', userDataDir];

		const install = spawnSync(cli, [...baseArgs, ...profile, '--install-extension', vsix], {
			stdio: 'inherit',
			shell: process.platform === 'win32',
		});
		if (install.status !== 0) {
			throw new Error(`\`code --install-extension\` exited ${install.status}`);
		}

		// Installing and *being installed* are different claims: `vsce` will
		// happily package a manifest VS Code then refuses, and the install
		// command's exit code has been known to lead. Ask the editor.
		const listed = spawnSync(cli, [...baseArgs, ...profile, '--list-extensions'], {
			encoding: 'utf8',
			shell: process.platform === 'win32',
		});
		const ids = (listed.stdout ?? '').split(/\r?\n/).map((s) => s.trim());
		if (!ids.some((id) => id.endsWith('.wolf'))) {
			throw new Error(
				`the extension is not listed after installing. --list-extensions said:\n${listed.stdout}`,
			);
		}
		console.log(`ok    vsix installs and lists as ${ids.find((id) => id.endsWith('.wolf'))}`);
	} finally {
		fs.rmSync(extensionsDir, { recursive: true, force: true });
		fs.rmSync(userDataDir, { recursive: true, force: true });
	}
}

main().catch((err) => {
	console.error(err instanceof Error ? err.message : err);
	process.exit(1);
});
