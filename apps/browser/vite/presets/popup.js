import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';

export function popupConfig({ browser, outDir, emptyOutDir }) {
	return defineConfig({
		define: {
			__BROWSER__: JSON.stringify(browser),
		},
		plugins: [
			sveltekit(),
			// your “special browser adapter” plugin for popup goes here
		],
	});
}
