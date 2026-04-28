#!/usr/bin/env node
import { spawnSync } from 'node:child_process';

if (process.env.CI === 'true') {
	console.log('CI=true: skipping mobile frontend build (handled by build-sveltekit-mobile job)');
	process.exit(0);
}

const result = spawnSync('pnpm', ['--filter', '@eurora/mobile', 'build'], {
	stdio: 'inherit',
	shell: process.platform === 'win32',
});

if (result.error) {
	console.error(result.error);
	process.exit(1);
}
process.exit(result.status ?? 1);
