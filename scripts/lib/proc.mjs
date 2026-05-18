// Thin wrappers around child_process for the dev scripts.
//
// `run` captures stdout/stderr as utf-8 strings and never throws — callers
// branch on `code`. ENOENT (binary missing from PATH) is normalized to
// code 127, the conventional "command not found" exit code, so callers
// can treat "not installed" and "installed but failed" uniformly.
//
// `runInherit` streams the child's stdio straight through, used when we
// want the user to see compile output / docker pull progress in real
// time (e.g. running the seed container).
//
// Windows + shell shims: Node's bare `CreateProcess` path only launches
// real `.exe` / `.com` files; `.cmd` / `.bat` shims (pnpm, yarn, npx,
// anything Corepack installs) ENOENT without `shell: true`, and since
// Node 20 modern releases reject `.cmd` paths outright via EINVAL even
// when given the full resolved path. The fix is per-call: pass
// `viaShell: true` when probing a binary whose installer might have
// produced a shim. We don't set it globally because cmd.exe quoting
// then mangles args containing spaces, parens, or quotes (e.g. the SQL
// strings in `seed-if-empty.mjs`).

import { spawn, spawnSync } from 'node:child_process';
import { constants as osConstants } from 'node:os';

const onWindows = process.platform === 'win32';

export function run(cmd, args = [], { viaShell = false, ...options } = {}) {
	const result = spawnSync(cmd, args, {
		encoding: 'utf8',
		shell: viaShell && onWindows,
		...options,
	});
	if (result.error) {
		// ENOENT is the dominant case: the binary isn't on PATH. Map it
		// to 127 so callers can use a single `code !== 0` branch.
		const code = result.error.code === 'ENOENT' ? 127 : 1;
		return { code, stdout: '', stderr: String(result.error.message ?? '') };
	}
	return {
		code: result.status ?? 1,
		stdout: result.stdout ?? '',
		stderr: result.stderr ?? '',
	};
}

export function runInherit(cmd, args = [], { viaShell = false, ...options } = {}) {
	return new Promise((resolvePromise) => {
		const child = spawn(cmd, args, {
			stdio: 'inherit',
			shell: viaShell && onWindows,
			...options,
		});
		child.on('error', (err) => {
			// Surface the failure on stderr so it isn't silently swallowed
			// by the inherited stdio (the child never started, so it can't
			// have written anything itself).
			process.stderr.write(`${cmd}: ${err.message}\n`);
			resolvePromise(err.code === 'ENOENT' ? 127 : 1);
		});
		child.on('exit', (code, signal) => {
			if (signal) {
				resolvePromise(128 + (osConstants.signals[signal] ?? 0));
				return;
			}
			resolvePromise(code ?? 1);
		});
	});
}
