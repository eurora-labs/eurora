import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, loadEnv } from 'vite';
import devtoolsJson from 'vite-plugin-devtools-json';

export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, process.cwd(), '');

	if (!env.VITE_API_BASE_URL) {
		throw new Error(
			'Missing required environment variable: VITE_API_BASE_URL\n' +
				'Please ensure this variable is set in your .env file or environment.',
		);
	}

	return {
		// server: {
		// 	port: 3000,
		// },
		plugins: [sveltekit(), devtoolsJson()],
		// server: {
		// 	watch: {
		// 		// Watch the UI package dist folder for changes
		// 		ignored: ['!**/node_modules/@eurora/ui/dist/**'],
		// 	},
		// },
		optimizeDeps: {
			exclude: ['@eurora/ui'],
		},
	};
});
