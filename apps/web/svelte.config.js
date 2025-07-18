import adapterStatic from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess({ script: true }),
	kit: {
		// adapter-auto only supports some environments, see https://kit.svelte.dev/docs/adapter-auto for a list.
		// If your environment is not supported or you settled on a specific environment, switch out the adapter.
		// See https://kit.svelte.dev/docs/adapters for more information about adapters.
		adapter: adapterStatic({
			pages: 'dist',
			assets: 'dist',
			fallback: '200.html',
			precompress: true,
			strict: true,
		}),
		paths: {
			// Ensure assets are loaded correctly on GitHub Pages
			assets: '',
		},
	},
	compilerOptions: {
		css: 'injected',
	},
};

export default config;
