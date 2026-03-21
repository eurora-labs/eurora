import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, loadEnv } from 'vite';
import devtoolsJson from 'vite-plugin-devtools-json';

export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, process.cwd(), '');

	if (!env.VITE_GRPC_API_URL) {
		throw new Error(
			'Missing required environment variable: VITE_GRPC_API_URL\n' +
				'Please ensure this variable is set in your .env file or environment.',
		);
	}

	if (!env.VITE_REST_API_URL) {
		console.warn(
			'VITE_REST_API_URL is not set — falling back to VITE_GRPC_API_URL (%s)',
			env.VITE_GRPC_API_URL,
		);
		process.env.VITE_REST_API_URL = env.VITE_GRPC_API_URL;
	}

	return {
		plugins: [sveltekit(), devtoolsJson()],
		optimizeDeps: {
			exclude: ['@eurora/ui'],
		},
	};
});
