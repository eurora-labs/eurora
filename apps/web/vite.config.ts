import { sentrySvelteKit } from '@sentry/sveltekit';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, loadEnv } from 'vite';
import devtoolsJson from 'vite-plugin-devtools-json';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, workspaceRoot, '');

	if (!env.VITE_API_URL) {
		throw new Error(
			'Missing required environment variable: VITE_API_URL\n' +
				'Please ensure this variable is set in your .env file or environment.',
		);
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
		envDir: workspaceRoot,
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
