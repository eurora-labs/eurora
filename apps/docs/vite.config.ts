import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
	plugins: [sveltekit()],
	resolve: {
		alias: {
			'@eurora/ui': path.resolve(__dirname, '../../packages/ui/src'),
			'@eurora/launcher': path.resolve(__dirname, '../../packages/custom-components/launcher/src')
		}
	}
});
