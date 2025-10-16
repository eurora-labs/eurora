/// <reference types='vitest' />
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';
import * as path from 'path';
import { readdirSync, cpSync } from 'fs';

const rootDir = path.resolve(__dirname);
const chromiumOutDir = path.resolve(__dirname, '../../../extensions/chromium/scripts/content');
const firefoxOutDir = path.resolve(__dirname, '../../../extensions/firefox/scripts/content');
const sitesDir = path.resolve(__dirname, 'src/sites');

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

function patternsFor(id: string): string[] {
	if (id === '_default') return [];
	if (id.includes('*')) return [id];
	return [id, `*.${id}`];
}

function RegistryPlugin() {
	return {
		name: 'emit-site-registry',
		generateBundle(_: any, bundle: any) {
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

function CopyToFirefoxPlugin() {
	return {
		name: 'copy-to-firefox',
		closeBundle() {
			// Copy all files from chromium output to firefox output
			cpSync(chromiumOutDir, firefoxOutDir, { recursive: true });
			console.log(`✓ Copied content scripts to firefox extension folder`);
		},
	};
}

const siteEntries = listSiteEntries();

export default defineConfig({
	root: __dirname,
	build: {
		outDir: chromiumOutDir,
		emptyOutDir: true,
		lib: {
			entry: path.resolve(__dirname, 'src/index.ts'),
			formats: ['es'],
		},
		// lib: false,
		rollupOptions: {
			input: Object.fromEntries([
				['bootstrap', path.resolve(rootDir, 'src/bootstrap.ts')],
				...siteEntries.map((e) => [e.name, e.path]),
			]),
			output: {
				format: 'es',
				entryFileNames: (chunk: any) => {
					// bootstrap stays at root, site entries go into sites/[name]/
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
	plugins: [RegistryPlugin(), CopyToFirefoxPlugin()],
});
