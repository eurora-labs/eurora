// import { svelteTesting } from '@testing-library/svelte/vite';
import { sentryVitePlugin } from '@sentry/vite-plugin';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

const sentryAuthToken = process.env.SENTRY_AUTH_TOKEN;
const sentryOrg = process.env.SENTRY_ORG;
const sentryProject = process.env.SENTRY_PROJECT;
const sentryRelease = process.env.SENTRY_RELEASE;

const sentryUploadEnabled = Boolean(sentryAuthToken && sentryOrg && sentryProject);

export default defineConfig({
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
	envPrefix: ['VITE_', 'TAURI_'],

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
	optimizeDeps: {
		exclude: ['@eurora/ui'],
	},

	// test: {
	// 	workspace: [
	// 		{
	// 			extends: './vite.config.ts',
	// 			plugins: [svelteTesting()],

	// 			test: {
	// 				name: 'client',
	// 				environment: 'jsdom',
	// 				clearMocks: true,
	// 				include: ['src/**/*.svelte.{test,spec}.{js,ts}'],
	// 				exclude: ['src/lib/server/**'],
	// 				setupFiles: ['./vitest-setup-client.ts']
	// 			}
	// 		},
	// 		{
	// 			extends: './vite.config.ts',

	// 			test: {
	// 				name: 'server',
	// 				environment: 'node',
	// 				include: ['src/**/*.{test,spec}.{js,ts}'],
	// 				exclude: ['src/**/*.svelte.{test,spec}.{js,ts}']
	// 			}
	// 		}
	// 	]
	// },
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
