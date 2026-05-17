// import { svelteTesting } from '@testing-library/svelte/vite';
import { sentryVitePlugin } from '@sentry/vite-plugin';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, loadEnv } from 'vite';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

export default defineConfig(({ mode }) => {
	// Read the workspace-root .env so the desktop bundle and the Sentry
	// build plugin see the same values the rest of the stack does.
	// `loadEnv` only exposes vars matching `envPrefix` to client code,
	// so backend secrets never leak into the bundle.
	const env = loadEnv(mode, workspaceRoot, '');

	const sentryVars = {
		SENTRY_AUTH_TOKEN: env.SENTRY_AUTH_TOKEN,
		SENTRY_ORG: env.SENTRY_ORG,
		SENTRY_PROJECT: env.SENTRY_PROJECT,
		SENTRY_RELEASE: env.SENTRY_RELEASE,
	};

	// Fail closed: a partial config in CI almost always means a missing
	// secret in the workflow or a stripped pass-through in turbo.json.
	// Silently skipping the plugin produces a green build with no
	// uploaded sourcemaps — exactly the bug we're trying to prevent.
	const present = Object.values(sentryVars).filter(Boolean).length;
	const partial = present > 0 && present < Object.keys(sentryVars).length;
	if (partial && process.env.CI) {
		const missing = Object.entries(sentryVars)
			.filter(([, v]) => !v)
			.map(([k]) => k);
		throw new Error(
			`Sentry vite plugin partially configured in CI; missing: ${missing.join(', ')}. ` +
				`Set all four of SENTRY_AUTH_TOKEN, SENTRY_ORG, SENTRY_PROJECT, SENTRY_RELEASE ` +
				`(via the workflow's step env and turbo.json passThroughEnv) or none.`,
		);
	}

	const sentryUploadEnabled = present === Object.keys(sentryVars).length;

	return {
		envDir: workspaceRoot,
		envPrefix: ['VITE_', 'TAURI_'],
		plugins: [
			debounceReload(),
			sveltekit(),
			...(sentryUploadEnabled
				? [
						sentryVitePlugin({
							authToken: sentryVars.SENTRY_AUTH_TOKEN,
							org: sentryVars.SENTRY_ORG,
							project: sentryVars.SENTRY_PROJECT,
							release: { name: sentryVars.SENTRY_RELEASE },
							sourcemaps: { assets: ['./build/**/*'] },
							telemetry: false,
						}),
					]
				: []),
		],

		server: {
			port: 1420,
			strictPort: false,
			fs: {
				strict: false,
			},
		},

		build: {
			rollupOptions: { output: { manualChunks: {} } },
			// Tauri supports es2021
			target: 'modules',
			// minify production builds
			minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
			// Source maps are required for the Sentry vite plugin to map
			// stack traces back to the original Svelte/TS sources.
			sourcemap: true,
		},
		worker: {
			// The Shiki highlighter worker dynamically imports per-language
			// grammar modules; Vite's default `iife` worker format can't
			// code-split, so we ship a real ES module worker instead.
			format: 'es',
		},
		optimizeDeps: {
			exclude: ['@eurora/ui'],
		},
	};
});

function debounceReload() {
	let timeout: NodeJS.Timeout | undefined;
	let mustReload = false;
	let longDelay = false;

	return {
		name: 'debounce-reload',
		/**
		 * There is a `handleHotUpdate` callback that has the same docs, and
		 * gets called as expected, but that fails to prevent the reload.
		 */
		hotUpdate({ server, file }: { server: any; file: string }) {
			if (!file.includes('apps/desktop')) {
				mustReload = true;
				longDelay = true;
			} else if (file.includes('.svelte-kit')) {
				mustReload = true;
			}
			if (mustReload) {
				clearTimeout(timeout);
				timeout = setTimeout(
					() => {
						timeout = undefined;
						mustReload = false;
						longDelay = false;
						server.hot.send({ type: 'full-reload' });
					},
					longDelay ? 5000 : 250,
				);
				server.hot.send('gb:reload');
				return []; // Prevent immediate reload.
			}
		},
	} as any;
}
