#!/usr/bin/env node
// First-run setup: copy `.env.example` to `.env` if `.env` doesn't exist yet.
// Idempotent — safe to re-run.
//
// Uses `COPYFILE_EXCL` so the "don't clobber an existing .env" guarantee
// is enforced atomically by the OS instead of via a stat-then-copy
// (which has a TOCTOU race, however unlikely in practice).

import { copyFile, constants as fsConstants } from 'node:fs/promises';

import { fromRepoRoot } from './lib/paths.mjs';

const src = fromRepoRoot('.env.example');
const dst = fromRepoRoot('.env');

try {
	await copyFile(src, dst, fsConstants.COPYFILE_EXCL);
	console.log('.env created from .env.example — open it and set OPENAI_API_KEY.');
} catch (err) {
	if (err.code === 'EEXIST') {
		console.log('.env already exists — leaving it alone.');
	} else {
		console.error(`Failed to create .env: ${err.message}`);
		process.exit(1);
	}
}
