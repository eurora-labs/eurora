// Read `.env` / `.env.example` for the dev scripts.
//
// `just` exports `.env` via `set dotenv-load` when it invokes a recipe,
// so any script launched through `just` sees the values in `process.env`
// already. The on-disk fallback exists so the scripts work when run
// standalone (e.g. `node scripts/doctor.mjs`).
//
// Parsing rules (kept intentionally simple — Just's dotenv-load is the
// authoritative source, this is a best-effort host probe):
//
//   - Skip blank lines and lines whose first non-whitespace character is `#`.
//   - Recognize `KEY=value` where KEY matches /^[A-Z_][A-Z0-9_]*$/.
//   - If the value is wrapped in matching single or double quotes, strip them.
//   - No interpolation, no escape sequences, no multi-line values.

import { readFileSync } from 'node:fs';

const KEY_LINE = /^\s*([A-Z_][A-Z0-9_]*)\s*=(.*)$/;

function readLines(filePath) {
	try {
		return readFileSync(filePath, 'utf8').split(/\r?\n/);
	} catch (err) {
		if (err.code === 'ENOENT') return null;
		throw err;
	}
}

function unquote(raw) {
	const v = raw.trim();
	if (v.length >= 2) {
		const first = v[0];
		const last = v[v.length - 1];
		if ((first === '"' || first === "'") && first === last) {
			return v.slice(1, -1);
		}
	}
	return v;
}

// Look up `key` in a dotenv-format file. Returns null if the file
// doesn't exist or the key isn't present. An empty string in the file
// is returned as an empty string (distinguishable from "absent").
export function readDotenvKey(filePath, key) {
	const lines = readLines(filePath);
	if (lines === null) return null;
	for (const line of lines) {
		const m = line.match(KEY_LINE);
		if (m && m[1] === key) return unquote(m[2]);
	}
	return null;
}

// Resolve `key`'s effective value: process env wins (matching the
// `dotenv-load` precedence Just uses), with a fallback to scanning the
// on-disk `.env` for standalone invocation. Returns an empty string
// when neither source defines it.
export function resolveEnv(key, dotenvPath) {
	const fromProcess = process.env[key];
	if (fromProcess !== undefined && fromProcess !== '') return fromProcess;
	const fromFile = readDotenvKey(dotenvPath, key);
	return fromFile ?? '';
}

// Return every KEY defined (uncommented) in `.env.example`. This is the
// canonical list of "required keys" — adding a new variable to the
// project means uncommenting it in `.env.example`, and the doctor
// picks it up automatically.
export function parseRequiredKeys(dotenvExamplePath) {
	const lines = readLines(dotenvExamplePath);
	if (lines === null) return [];
	const keys = [];
	for (const line of lines) {
		const m = line.match(KEY_LINE);
		if (m) keys.push(m[1]);
	}
	return keys;
}
