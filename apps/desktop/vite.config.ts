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

	const sentryAuthToken = env.SENTRY_AUTH_TOKEN;
	const sentryOrg = env.SENTRY_ORG;
	const sentryProject = env.SENTRY_PROJECT;
	const sentryRelease = env.SENTRY_RELEASE;

	const sentryUploadEnabled = Boolean(sentryAuthToken && sentryOrg && sentryProject);

	return {
		envDir: workspaceRoot,
		envPrefix: ['VITE_', 'TAURI_'],
		plugins: [
			debounceReload(),
			sveltekit(),
			...(sentryUploadEnabled
				? [
						sentryVitePlugin({
							authToken: sentryAuthToken,
							org: sentryOrg,
							project: sentryProject,
							release: sentryRelease ? { name: sentryRelease } : undefined,
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
