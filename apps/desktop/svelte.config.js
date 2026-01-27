import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	// preprocess: [svelteInjectComment(), vitePreprocess({ script: true })],
	preprocess: [vitePreprocess({ script: true })],

	kit: {
		alias: {
			$styles: './src/styles',
			$components: './src/lib/components',
		},
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: 'index.html',
			precompress: true,
			strict: false,
		}),
	},
	compilerOptions: {
		css: 'injected',
	},
};

export default config;
