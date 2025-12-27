import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export function popupConfig({ browser }) {
	return defineConfig({
		define: {
			__BROWSER__: JSON.stringify(browser),
		},
		plugins: [
			sveltekit(),
			// your "special browser adapter" plugin for popup goes here
		],
	});
}
