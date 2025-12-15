import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export function popupConfig({ browser, outDir, emptyOutDir }) {
	return defineConfig({
		define: {
			__BROWSER__: JSON.stringify(browser),
		},
		plugins: [
			svelte(),
			// your “special browser adapter” plugin for popup goes here
		],
		build: {
			outDir,
			emptyOutDir,
			rollupOptions: {
				input: {
					popup: 'src/popup/app.html',
				},
			},
		},
	});
}
