import { sentrySvelteKit } from '@sentry/sveltekit';
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

	const sentryAuthToken = env.SENTRY_AUTH_TOKEN;
	const sentryOrg = env.SENTRY_ORG;
	const sentryProject = env.SENTRY_PROJECT;
	const sentryRelease = env.PUBLIC_SENTRY_RELEASE;
	const uploadSourceMaps = Boolean(sentryAuthToken && sentryOrg && sentryProject);

	if (uploadSourceMaps && !sentryRelease) {
		throw new Error(
			'Missing PUBLIC_SENTRY_RELEASE. It is required when uploading source maps so ' +
				'the runtime release matches the uploaded artifacts.',
		);
	}

	return {
		plugins: [
			sentrySvelteKit({
				autoUploadSourceMaps: uploadSourceMaps,
				sourceMapsUploadOptions: uploadSourceMaps
					? {
							org: sentryOrg,
							project: sentryProject,
							authToken: sentryAuthToken,
							release: { name: sentryRelease },
						}
					: undefined,
			}),
			sveltekit(),
			devtoolsJson(),
		],
		optimizeDeps: {
			exclude: ['@eurora/ui'],
		},
	};
});
