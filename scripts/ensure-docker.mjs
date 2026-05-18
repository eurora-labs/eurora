#!/usr/bin/env node
// Ensure the Docker daemon is reachable before `just dev` runs the
// doctor. Doctor itself is side-effect-free by contract — this script
// is the place where we're allowed to *act*.
//
// Behavior:
//   - daemon already up    → exit 0 immediately
//   - macOS, daemon down   → `open -a Docker`, then poll until ready
//   - Windows, daemon down → launch Docker Desktop.exe, then poll until ready
//   - Linux, daemon down   → exit 0 (starting it needs sudo; doctor
//                            will surface the failure with a hint)
//   - docker not installed → exit 0 (doctor will report it)
//
// Idempotent. Cheap on the happy path (one `docker info` call).

import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

import { run } from './lib/proc.mjs';

const DEFAULT_TIMEOUT_SECS = 90;

function dockerInstalled() {
	return run('docker', ['--version']).code === 0;
}

function dockerDaemonUp() {
	return run('docker', ['info']).code === 0;
}

// Find Docker Desktop's launcher on Windows. Returns null if the
// install isn't where we expect and PATH doesn't know about it.
function findDockerDesktopWindows() {
	const candidates = [
		join(process.env.ProgramFiles ?? '', 'Docker', 'Docker', 'Docker Desktop.exe'),
		join(process.env.LOCALAPPDATA ?? '', 'Programs', 'Docker', 'Docker', 'Docker Desktop.exe'),
	].filter(Boolean);
	for (const c of candidates) {
		if (existsSync(c)) return c;
	}
	const where = run('where', ['Docker Desktop.exe']);
	if (where.code === 0) {
		const first = where.stdout.split(/\r?\n/).find((l) => l.trim().length > 0);
		if (first) return first.trim();
	}
	return null;
}

function startDockerDesktop() {
	if (process.platform === 'darwin') {
		return run('open', ['-a', 'Docker']).code === 0;
	}
	if (process.platform === 'win32') {
		const exe = findDockerDesktopWindows();
		if (!exe) return false;
		// Detach so this script doesn't hold a handle to the GUI process.
		const child = spawn(exe, [], { detached: true, stdio: 'ignore' });
		child.unref();
		return true;
	}
	return false;
}

if (!dockerInstalled()) process.exit(0);
if (dockerDaemonUp()) process.exit(0);

if (process.platform !== 'darwin' && process.platform !== 'win32') {
	// Linux / other: doctor will surface the failure with a remediation
	// hint. Starting the daemon would need sudo, which we can't do here.
	process.exit(0);
}

console.log('Docker daemon not running — starting Docker Desktop…');
if (!startDockerDesktop()) {
	// Couldn't find or launch the GUI. Bail quietly; doctor will report
	// the unreachable daemon next.
	process.exit(0);
}

const deadlineSecs = Number(process.env.EURORA_DOCKER_TIMEOUT_SECS) || DEFAULT_TIMEOUT_SECS;
const deadline = Date.now() + deadlineSecs * 1000;
while (!dockerDaemonUp()) {
	if (Date.now() >= deadline) {
		console.error(`Docker did not become ready within ${deadlineSecs}s.`);
		process.exit(1);
	}
	await sleep(1000);
}
console.log('Docker is ready.');
