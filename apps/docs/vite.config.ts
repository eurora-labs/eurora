import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
	plugins: [sveltekit()],
	resolve: {
		alias: {
			'@eurora/launcher': path.resolve(
				__dirname,
				'../../packages/custom-components/launcher/src'
			)
		}
	}
});
