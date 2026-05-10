import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [sveltekit()],
	test: {
		include: ['src/**/*.(test|spec).?(m)[jt]s?(x)'],
	},
	build: {
		sourcemap: 'inline',
	},
	worker: {
		// The Shiki highlighter worker dynamically imports per-language
		// grammar modules; Vite's default `iife` worker format can't
		// code-split, so we ship a real ES module worker instead.
		format: 'es',
	},
});
