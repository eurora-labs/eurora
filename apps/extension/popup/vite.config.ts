import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
	plugins: [sveltekit()],
	build: {
		emptyOutDir: true,
		reportCompressedSize: true,
		commonjsOptions: {
			transformMixedEsModules: true,
		},
		cssCodeSplit: true,
	},
});
