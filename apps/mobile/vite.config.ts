import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
	plugins: [sveltekit()],

	server: {
		host: host || false,
		port: 1421,
		strictPort: true,
		fs: {
			strict: false,
		},
	},
	envPrefix: ['VITE_', 'TAURI_'],

	build: {
		target: 'modules',
		minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
		sourcemap: true,
	},
});
