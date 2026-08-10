// The Wolf extension: an LSP client, and deliberately nothing else.
//
// D34 in one file. `wolf lsp` is the compiler, so there is no server to
// download, no version to reconcile, and no capability this extension could
// usefully implement itself. Everything a user sees in a `.lu` buffer —
// diagnostics, hover, document symbols, formatting, code actions — arrives
// because `vscode-languageclient` registered a provider for a capability the
// SERVER advertised. This file contains no provider, no `registerHoverProvider`,
// no diagnostic collection of its own.
//
// That is not minimalism for its own sake. D22 makes wolf's diagnostic catalog
// the reviewed product: a client that rewrites a message, remaps a severity, or
// filters a code becomes a second, unreviewed authority on what the compiler
// said. The way to not be that is to have nowhere to put the code.

import * as cp from 'child_process';
import * as vscode from 'vscode';
import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind,
} from 'vscode-languageclient/node';

import { COMPAT } from './compat';
import { PIN } from './pin';

/// The install instruction that is actually true. There is no marketplace
/// listing and no published wolf release, so the repository README is the whole
/// story — and a notification linking to a page nobody has written would be
/// worse than one linking nowhere.
const INSTALL_URL = 'https://github.com/tenseleyFlow/wolf-lsp#installing';

let client: LanguageClient | undefined;

/// The out-of-range warning fires **at most once per session** (ls07 §3).
///
/// Module scope, not `ExtensionContext` state: `wolf.restartServer` and a
/// `wolf.serverPath` change both re-enter `start()`, and a user who is already
/// mid-way through fixing their toolchain does not need the same modal-free
/// nag on every restart. It resets when the extension host does, which is the
/// definition of "per session" that a user would recognise.
let compatWarned = false;

/// The output channel is created once and OUTLIVES the client, because
/// `Wolf: Show Server Log` has to work after a crash — which is the moment
/// someone reaches for it. A channel owned by the client disappears with it.
let channel: vscode.OutputChannel | undefined;

/// The channel, or a loud failure. Everything below runs after `activate`, so
/// an absent channel is a bug in this file rather than a state to tolerate --
/// and `outputChannel?: OutputChannel` under `exactOptionalPropertyTypes` will
/// not accept a maybe.
function out(): vscode.OutputChannel {
	if (channel === undefined) {
		throw new Error('wolf: the output channel is used before activate()');
	}
	return channel;
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
	channel = vscode.window.createOutputChannel('Wolf Language Server');
	context.subscriptions.push(channel);

	context.subscriptions.push(
		vscode.commands.registerCommand('wolf.restartServer', restart),
		vscode.commands.registerCommand('wolf.showServerLog', () => out().show(true)),
		vscode.commands.registerCommand('wolf.showVersion', showVersion),
	);

	// A `wolf.serverPath` change is the one setting that cannot take effect
	// without a restart, so it restarts itself rather than leaving the user
	// with a stale process and a setting that appears to have done nothing.
	context.subscriptions.push(
		vscode.workspace.onDidChangeConfiguration(async (e) => {
			if (e.affectsConfiguration('wolf.serverPath')) {
				await restart();
			}
		}),
	);

	await start();
}

export async function deactivate(): Promise<void> {
	await stop();
}

// ------------------------------------------------------------- discovery --

/// The configured path, or `wolf` on `PATH`.
///
/// Deliberately NOT resolved to an absolute path here: letting the OS do the
/// `PATH` lookup at spawn time is what makes the capture proxy in
/// `clients/vscode/README.md` work (a shim named `wolf` earlier on `PATH`),
/// and it is what a user expects when they switch toolchains in a shell.
export function serverCommand(): string {
	const configured = vscode.workspace.getConfiguration('wolf').get<string>('serverPath', '');
	return configured.trim() === '' ? 'wolf' : configured.trim();
}

/// `wolf --version`, or `undefined` when the binary will not run.
///
/// Spawned with `--version` rather than probed with a `which`: resolving the
/// name is not the question, running it is, and a binary that resolves but
/// cannot execute must read as absent rather than as a mysterious failure
/// thirty seconds later. Same reasoning as `tests/nvim_plugin.rs`'s `nvim()`.
export function probeVersion(command: string): string | undefined {
	try {
		const out = cp.execFileSync(command, ['--version'], {
			encoding: 'utf8',
			timeout: 5000,
			windowsHide: true,
		});
		return out.trim();
	} catch {
		return undefined;
	}
}

// ------------------------------------------------------ the wolf range --

/// Where a version string sits relative to the range `compat.json` declares.
///
/// Exported and pure so the extension suite can drive the two states no machine
/// running these tests can produce — a wolf below `min` and a wolf above
/// `max_tested` — without a stand-in binary. There is exactly one wolf build in
/// existence, so both boundaries are unreachable any other way.
///
/// Numeric, not lexical: `0.10.0 < 0.9.0` as strings, and a check that gets
/// that backwards warns on precisely the upgrades it should stay quiet about.
export type Verdict = 'in-range' | 'below' | 'above' | 'unparseable';

export function versionTriple(versionString: string): [number, number, number] | undefined {
	for (const word of versionString.split(/\s+/)) {
		const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(word);
		if (m) {
			return [Number(m[1]), Number(m[2]), Number(m[3])];
		}
	}
	return undefined;
}

export function versionVerdict(versionString: string): Verdict {
	const found = versionTriple(versionString);
	const min = versionTriple(COMPAT.min);
	const max = versionTriple(COMPAT.maxTested);
	if (found === undefined || min === undefined || max === undefined) {
		return 'unparseable';
	}
	const cmp = (a: [number, number, number], b: [number, number, number]): number => {
		for (let i = 0; i < 3; i++) {
			if (a[i]! !== b[i]!) {
				return a[i]! < b[i]! ? -1 : 1;
			}
		}
		return 0;
	};
	if (cmp(found, min) < 0) {
		return 'below';
	}
	if (cmp(found, max) > 0) {
		return 'above';
	}
	return 'in-range';
}

/// The human-readable range, collapsed when it is one version wide — which it
/// is today, and will be until wolf-lang publishes a second release.
export function declaredRange(): string {
	return COMPAT.min === COMPAT.maxTested
		? `exactly ${COMPAT.min}`
		: `${COMPAT.min} .. ${COMPAT.maxTested}`;
}

/// One message, once, naming both versions and the upgrade path — and then the
/// server starts anyway (ls07 §3).
///
/// No modal, no repetition, no auto-update, no refusal to run. An out-of-range
/// server usually mostly works, and blocking someone's editor over a version
/// comparison is a worse outcome than a notification they dismiss. The word
/// "unsupported" does not appear: nobody set that policy.
///
/// The output channel gets the line unconditionally, warning or not, because
/// the channel is where someone looks *after* dismissing the toast.
function warnIfOutOfRange(version: string): void {
	const verdict = versionVerdict(version);
	const range = declaredRange();
	out().appendLine(
		`wolf: found ${version}; this extension declares wolf ${range} (verified ${COMPAT.verified}) — ${verdict}`,
	);
	if (verdict === 'in-range' || compatWarned) {
		return;
	}
	compatWarned = true;

	const detail =
		verdict === 'above'
			? 'newer than any wolf this extension has been tested against'
			: verdict === 'below'
				? 'older than the oldest wolf this extension declares'
				: 'not a version string this extension can compare';
	void vscode.window
		.showWarningMessage(
			`Wolf: \`${version}\` is ${detail} (declared range: wolf ${range}). ` +
				`Editing is unaffected — this is a warning, not a refusal.`,
			'Show Server Log',
			'Compatibility',
		)
		.then((choice) => {
			if (choice === 'Show Server Log') {
				out().show(true);
			} else if (choice === 'Compatibility') {
				void vscode.env.openExternal(vscode.Uri.parse(COMPAT_URL));
			}
		});
}

/// `docs/COMPAT.md` on the repository, which is the only place the range is
/// explained — there is no published documentation site.
const COMPAT_URL = 'https://github.com/tenseleyFlow/wolf-lsp/blob/trunk/docs/COMPAT.md';

// ----------------------------------------------------------- the client --

async function start(): Promise<void> {
	const command = serverCommand();
	const version = probeVersion(command);

	if (version === undefined) {
		// ONE notification, non-modal, no retry loop, no auto-download.
		// `showWarningMessage` returns a promise nobody awaits: blocking
		// activation on a user's attention is how an extension becomes the
		// reason a window takes ten seconds to open.
		void vscode.window
			.showWarningMessage(
				`Wolf: no \`${command}\` executable found. Syntax highlighting works; ` +
					`diagnostics, hover, formatting and code actions need the toolchain.`,
				'Install wolf',
				'Open Settings',
			)
			.then((choice) => {
				if (choice === 'Install wolf') {
					void vscode.env.openExternal(vscode.Uri.parse(INSTALL_URL));
				} else if (choice === 'Open Settings') {
					void vscode.commands.executeCommand(
						'workbench.action.openSettings',
						'wolf.serverPath',
					);
				}
			});
		out().appendLine(
			`wolf: \`${command} --version\` did not run; the language server was not started.`,
		);
		return;
	}

	// Before the client starts, so the line is already in the channel if the
	// handshake then fails — a version mismatch is the first thing anyone would
	// want to see in that log, and a message emitted after a throw is not there.
	warnIfOutOfRange(version);

	const serverOptions: ServerOptions = {
		command,
		args: ['lsp'],
		transport: TransportKind.stdio,
	};

	const clientOptions: LanguageClientOptions = {
		// `.lu` only, and that is a deliberate narrowing of the sprint's
		// parenthetical — see `clients/vscode/README.md`, "What `.wolfi` gets".
		// `wolf lsp` has no `.wolfi` path at this pin (the format is BINARY:
		// `WOLFI` magic bytes, `upstream/crates/wolf_sema/src/interface.rs`),
		// and attaching a client to documents the server ignores produces a
		// buffer that looks supported and is not.
		documentSelector: [{ scheme: 'file', language: 'wolf' }],
		outputChannel: out(),
		// Diagnostics render as sent (D22). No middleware, no
		// `handleDiagnostics` hook — the two places a client could quietly
		// become a second authority on the catalog.
	};

	client = new LanguageClient('wolf', 'Wolf Language Server', serverOptions, clientOptions);

	// NOTHING trims the client capabilities. `vscode-languageclient` declares
	// what it declares; a hand-edited capability set is a claim about this
	// editor that this editor did not make, and `profiles/vscode.json` is
	// read off the wire precisely so nobody has to take one on faith.
	await client.start();
}

async function stop(): Promise<void> {
	const running = client;
	client = undefined;
	if (running) {
		await running.stop();
	}
}

async function restart(): Promise<void> {
	await stop();
	await start();
}

// ------------------------------------------------------------- commands --

/// What is installed, what this extension declares, and which side of the range
/// the binary is on.
///
/// The extension does NOT refuse to start against a version outside its range
/// and does not call one "unsupported" — that is a policy nobody set, and an
/// out-of-range server usually mostly works. Reporting it and getting out of
/// the way is the behaviour `:checkhealth wolf` settled on for the same reason.
///
/// Pre-1.0 the range is one version wide because wolf-lang tags no releases and
/// the suite has been run against exactly one build. `docs/COMPAT.md` states
/// that posture rather than implying a stability this track cannot provide.
async function showVersion(): Promise<void> {
	const command = serverCommand();
	const version = probeVersion(command);
	const lines = [
		`extension    ${extensionVersion()}`,
		`wolf         ${version ?? `not found (\`${command}\` did not run)`}`,
		`declares     wolf ${declaredRange()} (verified ${COMPAT.verified})`,
		`verified at  ${PIN.version} (wolf-lang ${PIN.commit.slice(0, 7)})`,
		`server       ${client ? 'running' : 'not running'}`,
	];
	if (version !== undefined) {
		const verdict = versionVerdict(version);
		lines.push(`range        ${verdict}`);
		if (verdict === 'above') {
			lines.push(
				'',
				'This wolf is newer than anything the conformance suite has been run',
				'against. Usually fine — the extension only sends standard LSP — but',
				'nothing verified this combination. Update the extension, or report',
				'what broke: https://github.com/tenseleyFlow/wolf-lsp/issues',
			);
		} else if (verdict === 'below') {
			lines.push(
				'',
				'This wolf is older than the floor the extension declares. Update the',
				'toolchain, or install an extension version whose range covers it.',
			);
		} else if (verdict === 'unparseable') {
			lines.push(
				'',
				'That string carries no MAJOR.MINOR.PATCH, so no range comparison was',
				'made. Check what `wolf.serverPath` points at — it may not be wolf.',
			);
		}
	}
	out().appendLine(lines.join('\n'));
	out().show(true);
}

function extensionVersion(): string {
	return vscode.extensions.getExtension('wolf-lang-unpublished.wolf')?.packageJSON?.version ?? '(unknown)';
}
