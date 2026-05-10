import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [sveltekit()],
	test: {
		include: ['src/**/*.(test|spec).?(m)[jt]s?(x)'],
		// Tests touch reactive Svelte primitives (`$state`, `$effect.root`,
		// runed's `watch`), which need Svelte's client runtime and a DOM.
		// jsdom + the `browser` resolve condition pick `svelte/index-client.js`
		// over the noop server entry.
		environment: 'jsdom',
		server: {
			deps: {
				inline: ['runed'],
			},
		},
	},
	resolve: {
		conditions: ['browser'],
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
