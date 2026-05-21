#!/usr/bin/env node
// Block until the backend's /health endpoint responds, with a 300s ceiling
// to cover a slow first-time debug compile.
//
// Used by `just dev` (and friends) to delay web / desktop / mobile startup
// until the backend has bound its port — without this, Vite tries to call
// /llm/info before the backend exists and clients surface connection errors
// on boot.
//
// Cross-platform replacement for the previous wait-for-backend.{sh,ps1}
// pair: one file works identically on macOS, Linux, and Windows, and uses
// only Node's built-in `fetch` (Node ≥ 18). The Node engine requirement is
// already pinned in package.json#engines.
//
// Backend URL comes from $BACKEND_URL, which the Justfile already exports
// (defaulting to http://localhost:3000 and overridden by `just ios-device`
// to the LAN host). The localhost fallback below only matters for
// standalone invocation outside `just`.

const baseUrl = process.env.BACKEND_URL ?? 'http://localhost:3000';
const url = `${baseUrl.replace(/\/$/, '')}/health`;
const deadlineSecs = 300;
const deadline = Date.now() + deadlineSecs * 1000;
const pollIntervalMs = 500;

// Per-request timeout matches the polling cadence's order of magnitude
// without being so tight it fails under heavy compile load (the original
// PowerShell port used 2s and lost every poll while Tauri was compiling).
const perRequestTimeoutMs = 10_000;

async function probe() {
	const controller = new AbortController();
	const t = setTimeout(() => controller.abort(), perRequestTimeoutMs);
	try {
		const res = await fetch(url, { signal: controller.signal });
		return res.ok;
	} catch {
		return false;
	} finally {
		clearTimeout(t);
	}
}

while (Date.now() < deadline) {
	if (await probe()) {
		console.log('Backend is ready.');
		process.exit(0);
	}
	await new Promise((r) => setTimeout(r, pollIntervalMs));
}

console.error(`Backend did not become ready within ${deadlineSecs}s.`);
process.exit(1);
