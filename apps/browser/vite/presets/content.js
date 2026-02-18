import { readdirSync } from 'node:fs';
import path from 'node:path';

const sitesDir = path.resolve(import.meta.dirname, '../../src/content/sites');

function listSiteEntries() {
	return readdirSync(sitesDir, { withFileTypes: true })
		.filter((entry) => entry.isDirectory())
		.map((d) => d.name)
		.map((name) => ({
			name: name,
			path: path.resolve(sitesDir, `${name}/index.ts`),
			id: name,
		}));
}

function patternsFor(id) {
	if (id === '_default' || id === '_pdf') return [];
	if (id.includes('*')) return [id];
	return [id, `*.${id}`];
}

function RegistryPlugin() {
	return {
		name: 'emit-site-registry',
		generateBundle() {
			const entries = listSiteEntries()
				.filter((e) => e.id !== '_default')
				.map((e) => {
					const key = `sites/${e.name}/index.js`;
					return {
						id: e.id,
						chunk: key,
						patterns: patternsFor(e.id),
					};
				});
			const registry = JSON.stringify(entries, null, 2);
			this.emitFile({
				type: 'asset',
				fileName: 'registry.json',
				source: registry,
			});
		},
	};
}

/**
 * Build configuration for content scripts.
 *
 * The background script programmatically injects:
 * 1. scripts/content/bootstrap.js - the lightweight loader
 * 2. scripts/content/sites/{siteId}/index.js - site-specific handlers
 *
 * Site scripts are loaded dynamically via browser.runtime.getURL() and import()
 */
export function contentConfig({ browser, outDir, emptyOutDir, mode }) {
	const rootDir = path.resolve(import.meta.dirname, '../..');
	const siteEntries = listSiteEntries();

	return {
		configFile: false,
		mode: mode || 'production',
		root: rootDir,
		define: {
			__BROWSER__: JSON.stringify(browser),
			__DEV__: JSON.stringify(mode === 'development'),
		},
		plugins: [RegistryPlugin()],
		build: {
			outDir: path.join(outDir, 'scripts/content'),
			emptyOutDir,
			rollupOptions: {
				input: Object.fromEntries([
					['bootstrap', path.resolve(rootDir, 'src/content/bootstrap.ts')],
					...siteEntries.map((e) => [e.name, e.path]),
				]),
				output: {
					format: 'es',
					entryFileNames: (chunk) => {
						if (chunk.name === 'bootstrap') {
							return 'bootstrap.js';
						}
						return `sites/${chunk.name}/index.js`;
					},
					chunkFileNames: 'chunks/[name]-[hash].js',
					assetFileNames: 'assets/[name]-[hash][extname]',
					preserveModules: false,
					exports: 'named',
				},
				preserveEntrySignatures: 'exports-only',
				treeshake: false,
			},
			target: 'es2022',
			minify: 'esbuild',
			modulePreload: false,
			sourcemap: false,
			cssCodeSplit: true,
		},
	};
}
