/// <reference types='vitest' />
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';
import * as path from 'path';
import copy from 'rollup-plugin-copy';
import { readdirSync } from 'fs';

const rootDir = path.resolve(__dirname);
const outDir = path.resolve(__dirname, '../../../../extensions/chromium/scripts/content');
const sitesDir = path.resolve(__dirname, 'src/sites');

function listSiteEntries() {
	return readdirSync(sitesDir, { withFileTypes: true })
		.filter((entry) => entry.isDirectory())
		.map((d) => d.name)
		.map((name) => ({
			name: `sites/${name}/index`,
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
					const key = `${e.name}.js`;
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

const siteEntries = listSiteEntries();

export default defineConfig({
	root: __dirname,
	build: {
		outDir,
		emptyOutDir: false,
		rollupOptions: {
			input: Object.fromEntries([
				['bootstrap', path.resolve(rootDir)],
				...siteEntries.map((e) => [e.name, e.path]),
			]),
			output: {
				entryFileNames: (chunk: any) => `${chunk.name}.js`,
				chunkFileNames: 'chunks/[name]-[hash].js',
				assetFileNames: 'assets/[name]-[hash][extname]',
			},
		},
		target: 'es2022',
		minify: 'esbuild',
		modulePreload: false,
		sourcemap: false,
		cssCodeSplit: true,
	},
	plugins: [RegistryPlugin()],
});
