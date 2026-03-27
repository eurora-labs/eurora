import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],

	server: {
		port: 1421,
		strictPort: false,
		fs: {
			strict: false,
		},
	},
	envPrefix: ['VITE_', 'TAURI_'],

	build: {
		rollupOptions: { output: { manualChunks: {} } },
		target: 'modules',
		minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
		sourcemap: true,
	},
	optimizeDeps: {
		exclude: ['@eurora/ui'],
	},
});
