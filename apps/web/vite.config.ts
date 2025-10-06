import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import devtoolsJson from 'vite-plugin-devtools-json';

export default defineConfig({
	// server: {
	// 	port: 3000,
	// },
	plugins: [sveltekit(), devtoolsJson()],
	server: {
		watch: {
			// Watch the UI package dist folder for changes
			ignored: ['!**/node_modules/@eurora/ui/dist/**'],
		},
	},
	optimizeDeps: {
		exclude: ['@eurora/ui'],
	},
});
