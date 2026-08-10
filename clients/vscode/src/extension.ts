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

import { PIN } from './pin';

/// The docs page ls07 owns. Until it exists, the repository README is the
/// install instruction that is actually true, so that is what the notification
/// links to — a link to a page nobody has written is worse than no link.
const INSTALL_URL = 'https://github.com/tenseleyFlow/wolf-lsp#installing';

let client: LanguageClient | undefined;

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

// ----------------------------------------------------------- the client --

async function start(): Promise<void> {
	const command = serverCommand();

	if (probeVersion(command) === undefined) {
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

/// What is installed, what this extension was verified against, and no verdict.
///
/// The extension does NOT refuse to start against a version it does not
/// recognise, and does not call one "unsupported": a supported version RANGE is
/// ls07's to define, and the only honest fact this repository holds today is
/// one exact pin. Reporting the mismatch and getting out of the way is the
/// behaviour `:checkhealth wolf` settled on for the same reason.
async function showVersion(): Promise<void> {
	const command = serverCommand();
	const version = probeVersion(command);
	const lines = [
		`extension    ${extensionVersion()}`,
		`wolf         ${version ?? `not found (\`${command}\` did not run)`}`,
		`verified at  ${PIN.version} (wolf-lang ${PIN.commit.slice(0, 7)})`,
		`server       ${client ? 'running' : 'not running'}`,
	];
	if (version !== undefined && version !== PIN.version) {
		lines.push(
			'',
			'This is not the build the extension was verified against. That is not',
			'known to be a problem — the supported range is not defined yet (ls07).',
		);
	}
	out().appendLine(lines.join('\n'));
	out().show(true);
}

function extensionVersion(): string {
	return vscode.extensions.getExtension('wolf-lang-unpublished.wolf')?.packageJSON?.version ?? '(unknown)';
}
