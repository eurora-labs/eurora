/// <reference types='vitest' />
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';
import * as path from 'path';
import copy from 'rollup-plugin-copy';

export default defineConfig({
	root: __dirname,
	plugins: [
		dts({
			entryRoot: 'src',
			tsconfigPath: path.join(__dirname, 'tsconfig.json'),
		}),
	],
	// Configuration for building your library.
	// See: https://vitejs.dev/guide/build.html#library-mode
	build: {
		emptyOutDir: true,
		reportCompressedSize: true,
		commonjsOptions: {
			transformMixedEsModules: true,
		},
		lib: {
			// Could also be a dictionary or array of multiple entry points.
			entry: 'src/index.ts',
			name: 'content-script',
			fileName: 'index',
			// Change this to the formats you want to support.
			// Don't forget to update your package.json as well.
			formats: ['es'],
		},
		rollupOptions: {
			// External packages that should not be bundled into your library.
			external: [],
			output: {
				entryFileNames: 'main.js',
				chunkFileNames: 'chunks/[name]-[hash].js',
				assetFileNames: 'assets/[name]-[hash].[ext]',
			},
			plugins: [
				// @ts-expect-error - rollup-plugin-copy types are incompatible with Rollup 4
				copy({
					targets: [
						{
							src: 'dist/**/*',
							dest: '../../../extensions/chromium/scripts/content',
						},
						{
							src: 'dist/**/*',
							dest: '../../../extensions/firefox/scripts/content',
						},
					],
					hook: 'closeBundle',
					overwrite: true,
				}),
			],
		},
		target: 'es2022',
		minify: 'esbuild',
		modulePreload: false,
		sourcemap: false,
	},
	test: {
		watch: false,
		globals: true,
		environment: 'node',
		include: ['src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
		reporters: ['default'],
		coverage: {
			reportsDirectory: '../../../coverage/apps/content-scripts/article-watcher',
			provider: 'v8',
		},
	},
	resolve: {
		alias: {
			'@eurora/chrome-ext-shared/*': path.resolve(
				__dirname,
				'../../../../packages/chrome-ext-shared/src/lib/*',
			),
		},
	},
});
