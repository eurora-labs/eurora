import { sentrySvelteKit } from '@sentry/sveltekit';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, loadEnv, searchForWorkspaceRoot } from 'vite';
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

	// Bind the dev server to whatever host `WEB_URL` advertises, so the URL
	// the rest of the workspace points clients at is the URL the server
	// actually answers on. The default `WEB_URL=http://localhost:5173`
	// keeps the dev server loopback-only; `just ios-device` overrides
	// `WEB_URL` to a LAN IP so a physical iPhone on the same Wi-Fi can
	// reach the auth pages.
	const webUrl = new URL(env.WEB_URL ?? 'http://localhost:5173');
	const isLoopback = webUrl.hostname === 'localhost' || webUrl.hostname === '127.0.0.1';
	const serverHost = isLoopback ? false : webUrl.hostname;
	const serverPort = Number(webUrl.port) || 5173;

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

	// Production builds without a DSN ship without telemetry. That's a valid
	// choice (e.g. preview environments), but it's almost never intentional
	// in the main production deploy — surface it loudly in the build log so
	// a misconfiguration doesn't go unnoticed for weeks.
	if (mode === 'production' && !env.PUBLIC_SENTRY_WEB_DSN) {
		console.warn(
			'[sentry] PUBLIC_SENTRY_WEB_DSN is not set for this production build; ' +
				'no errors will be reported from either client or server.',
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
		server: {
			host: serverHost,
			port: serverPort,
			strictPort: true,
			fs: {
				// `@eurora/ui` ships a Shiki Web Worker loaded via
				// `new URL('./shiki-worker.js', import.meta.url)`, which Vite
				// resolves to the package's real on-disk path
				// (`packages/ui/dist/...`). That lives outside SvelteKit's
				// default `fs.allow` roots, so widen to the pnpm workspace
				// root to cover any first-party package that ships a
				// worker or asset URL.
				allow: [searchForWorkspaceRoot(process.cwd())],
			},
		},
		worker: {
			// The Shiki highlighter worker dynamically imports per-language
			// grammar modules; Vite's default `iife` worker format can't
			// code-split, so we ship a real ES module worker instead.
			format: 'es',
		},
	};
});
