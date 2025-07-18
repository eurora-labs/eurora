import staticAdapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess({ script: true }),
	kit: {
		adapter: staticAdapter({
			pages: 'dist',
			assets: 'dist',
			fallback: 'index.html',
			precompress: true,
			strict: false,
		}),
	},
	compilerOptions: {
		css: 'injected',

		runes: true,
	},
};

export default config;
