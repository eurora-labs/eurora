#!/usr/bin/env node
// Pre-flight check for `just dev`. Validates that the developer's machine
// has the tools and configuration we need before we try to bring the stack
// up. Exit code is 1 when any check failed and 0 otherwise, so the script
// fits cleanly into CI gates and `just dev: doctor` dependencies.
//
// Side-effect-free by design: nothing is installed or written. Failures
// carry a one-line remediation hint pointing at the exact command to run.
//
// Usage:
//   just doctor
//   node scripts/doctor.mjs   # equivalent

import { existsSync } from 'node:fs';
import { createConnection } from 'node:net';

import { parseRequiredKeys, resolveEnv } from './lib/env.mjs';
import { fromRepoRoot } from './lib/paths.mjs';
import { run } from './lib/proc.mjs';
import { color, glyph } from './lib/term.mjs';

const DOTENV = fromRepoRoot('.env');
const DOTENV_EXAMPLE = fromRepoRoot('.env.example');

// Column width for the check-name field. Tuned to fit the longest label
// without wrapping at 80 cols.
const LABEL_WIDTH = 18;

let failed = 0;

function pass(label, detail = '') {
	process.stdout.write(
		`  ${color.green(glyph.check)} ${label.padEnd(LABEL_WIDTH)} ${color.dim(detail)}\n`,
	);
}

function fail(label, detail = '', hint) {
	process.stdout.write(
		`  ${color.red(glyph.cross)} ${label.padEnd(LABEL_WIDTH)} ${color.red(detail)}\n`,
	);
	if (hint) process.stdout.write(`    ${color.dim(`${glyph.arrow} ${hint}`)}\n`);
	failed += 1;
}

// ─── Individual checks ─────────────────────────────────────────────────────

function checkCommand(label, cmd, installHint) {
	// `viaShell` lets us discover `.cmd` / `.bat` shims on Windows that
	// `CreateProcess` can't launch directly (pnpm via Corepack is the
	// canonical example). Safe here: `cmd` and the literal `--version`
	// arg are hardcoded — no shell metacharacters in scope.
	const r = run(cmd, ['--version'], { viaShell: true });
	if (r.code !== 0) {
		fail(label, 'not installed', installHint);
		return false;
	}
	const version = r.stdout.split(/\r?\n/)[0]?.trim() || 'unknown';
	pass(label, version);
	return true;
}

function checkDockerDaemon() {
	if (run('docker', ['info']).code === 0) {
		pass('docker daemon', 'running');
		return true;
	}
	const hint =
		process.platform === 'darwin'
			? "Start Docker Desktop: open -a Docker (or 'just dev' — auto-starts it)"
			: process.platform === 'win32'
				? "Start Docker Desktop (or 'just dev' — auto-starts it)."
				: process.platform === 'linux'
					? 'Start Docker: sudo systemctl start docker'
					: 'Start your Docker daemon.';
	fail('docker daemon', 'not reachable', hint);
	return false;
}

function checkDockerCompose() {
	if (run('docker', ['compose', 'version']).code !== 0) {
		fail(
			'docker compose',
			'v2 not found',
			"Update Docker; v1 'docker-compose' is unsupported.",
		);
		return false;
	}
	const v = run('docker', ['compose', 'version', '--short']).stdout.trim() || 'v2';
	pass('docker compose', v);
	return true;
}

// Returns true iff something is listening on 127.0.0.1:`port`.
//
// Uses a short TCP probe instead of OS-specific tools (`lsof`, `netstat`,
// `Test-NetConnection`) so the check is portable and fast. The 250ms
// timeout is comfortably above localhost connect latency (sub-ms) but
// short enough that the doctor doesn't drag on a misconfigured network.
function portInUse(port) {
	return new Promise((resolvePromise) => {
		const socket = createConnection({ host: '127.0.0.1', port });
		const finalize = (inUse) => {
			socket.destroy();
			resolvePromise(inUse);
		};
		socket.setTimeout(250);
		socket.once('connect', () => finalize(true));
		socket.once('timeout', () => finalize(true));
		socket.once('error', (err) => finalize(err.code !== 'ECONNREFUSED'));
	});
}

// True iff host port `port` is bound by our docker-compose Postgres
// container. `docker compose port` resolves the publish mapping
// directly (e.g. "0.0.0.0:5434"), which is more robust than scraping
// `docker ps`.
function portOwnedByEuroraPostgres(port) {
	const r = run('docker', ['compose', 'port', 'postgres', '5432']);
	if (r.code !== 0) return false;
	const binding = r.stdout.trim();
	if (!binding) return false;
	const boundPort = binding.split(':').pop();
	return boundPort === String(port);
}

async function checkPortFree(label, port, hintMsg) {
	if (await portInUse(port)) {
		fail(label, `in use (port ${port})`, hintMsg);
		return false;
	}
	pass(label, `free (port ${port})`);
	return true;
}

async function checkPostgresPort(port) {
	const label = `port ${port}`;
	if (!(await portInUse(port))) {
		pass(label, 'free');
		return true;
	}
	if (portOwnedByEuroraPostgres(port)) {
		pass(label, 'in use by Eurora postgres container');
		return true;
	}
	fail(
		label,
		'in use by something else',
		'Stop the conflicting process or change the host-side port in docker-compose.yml.',
	);
	return false;
}

function checkEnvFile() {
	if (existsSync(DOTENV)) {
		pass('.env', 'exists');
		return true;
	}
	fail('.env', 'not found', 'Run: just init');
	return false;
}

function checkEnvComplete() {
	if (!existsSync(DOTENV_EXAMPLE)) {
		fail('env vars', '.env.example not found at repo root');
		return false;
	}
	// OPENAI_API_KEY is excluded here because `checkOpenAiKey` runs a more
	// detailed check (placeholder detection) for it specifically.
	const required = parseRequiredKeys(DOTENV_EXAMPLE).filter((k) => k !== 'OPENAI_API_KEY');
	const missing = required.filter((k) => resolveEnv(k, DOTENV) === '');
	const total = required.length;

	if (missing.length === 0) {
		pass('env vars', `${total}/${total} required keys set`);
		return true;
	}

	const hintLines = [];
	if (missing.length <= 5) {
		hintLines.push(`Add to .env: ${missing.join(' ')}`);
	} else {
		hintLines.push('Run `just init` to create .env from .env.example, then re-run doctor.');
		hintLines.push(`Missing: ${missing.slice(0, 5).join(' ')} … (+${missing.length - 5} more)`);
	}
	fail('env vars', `${missing.length} of ${total} required key(s) missing`, hintLines[0]);
	for (const extra of hintLines.slice(1)) {
		process.stdout.write(`    ${color.dim(`${glyph.arrow} ${extra}`)}\n`);
	}
	return false;
}

function checkOpenAiKey() {
	const value = resolveEnv('OPENAI_API_KEY', DOTENV);
	if (value === '') {
		fail(
			'OPENAI_API_KEY',
			'unset',
			'Get a key from https://platform.openai.com/api-keys and add it to .env.',
		);
		return false;
	}
	if (value === 'sk-...' || value === 'sk_test') {
		fail(
			'OPENAI_API_KEY',
			'still set to a placeholder',
			'Replace the placeholder in .env with a real key from https://platform.openai.com/api-keys.',
		);
		return false;
	}
	pass('OPENAI_API_KEY', 'set');
	return true;
}

// ─── Main ──────────────────────────────────────────────────────────────────

process.stdout.write(`${color.bold('Eurora dev environment doctor')}\n`);
process.stdout.write(`${color.dim('─────────────────────────────')}\n`);

const dockerOk = checkCommand(
	'docker',
	'docker',
	'Install Docker Desktop or docker-engine: https://docs.docker.com/get-docker/',
);
if (dockerOk) {
	checkDockerDaemon();
	checkDockerCompose();
}

checkCommand('cargo', 'cargo', 'Install Rust via https://rustup.rs');
checkCommand('watchexec', 'watchexec', 'Install with: cargo install --locked watchexec-cli');
checkCommand('pnpm', 'pnpm', 'Install with: corepack enable');
checkCommand('just', 'just', 'Install with: cargo install just');

// Port checks. We resolve the backend port from the user's `BACKEND_URL`
// so the doctor follows whatever they've configured; the literal
// fallbacks (3000 / 5434) only fire when the variables are unset (e.g.
// a fresh checkout where doctor runs before `just init`) so the doctor
// itself stays usable in that broken state.
const backendUrl = resolveEnv('BACKEND_URL', DOTENV) || 'http://localhost:3000';
const httpPort = Number(backendUrl.split(':').pop().split('/')[0]) || 3000;
// The host port the postgres container binds on the host. Hardcoded in
// `docker-compose.yml` (5434) — the doctor matches that default.
const postgresPort = 5434;

await checkPortFree(
	`port ${httpPort}`,
	httpPort,
	'Stop the conflicting process or update BACKEND_URL.',
);
await checkPostgresPort(postgresPort);
await checkPortFree('port 5173', 5173, 'Stop the conflicting process or move the web dev server.');

checkEnvFile();
checkEnvComplete();
checkOpenAiKey();

process.stdout.write('\n');
if (failed > 0) {
	process.stdout.write(`${color.red(color.bold(`${failed} check(s) failed.`))}\n`);
	process.exit(1);
}
process.stdout.write(`${color.green(color.bold('All checks passed.'))}\n`);
