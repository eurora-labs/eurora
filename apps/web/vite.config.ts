import { sentrySvelteKit } from '@sentry/sveltekit';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, loadEnv } from 'vite';
import devtoolsJson from 'vite-plugin-devtools-json';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, workspaceRoot, '');

	if (!env.BACKEND_URL) {
		throw new Error(
			'Missing required environment variable: BACKEND_URL\n' +
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
		// Expose `BACKEND_URL` to client code as `import.meta.env.PUBLIC_API_URL`
		// without dragging it into the loader's `VITE_*` / `PUBLIC_*` namespace.
		// This is what the SvelteKit app reads via `ConfigService` — keeping
		// the env var name aligned with the workspace's other consumers
		// (be-monolith, the desktop build.rs scripts) means a single edit
		// to `.env` propagates everywhere.
		define: {
			'import.meta.env.PUBLIC_API_URL': JSON.stringify(env.BACKEND_URL),
		},
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
		worker: {
			// The Shiki highlighter worker dynamically imports per-language
			// grammar modules; Vite's default `iife` worker format can't
			// code-split, so we ship a real ES module worker instead.
			format: 'es',
		},
	};
});
