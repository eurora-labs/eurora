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
	build: {
		emptyOutDir: true,
		reportCompressedSize: true,
		commonjsOptions: {
			transformMixedEsModules: true,
		},
		lib: {
			// Could also be a dictionary or array of multiple entry points.
			entry: 'src/index.ts',
			name: 'youtube-watcher',
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
				chunkFileNames: 'main-[name].js',
				assetFileNames: 'assets/[name].[ext]',
			},
			plugins: [
				// @ts-expect-error
				copy({
					targets: [
						{
							src: 'dist/**/*',
							dest: '../../../../extensions/chromium/content-scripts/youtube-watcher',
						},
						{
							src: 'dist/**/*',
							dest: '../../../../extensions/firefox/content-scripts/youtube-watcher',
						},
					],
					hook: 'closeBundle',
					overwrite: true,
				}),
			],
		},
	},
	test: {
		watch: false,
		globals: true,
		environment: 'node',
		include: ['src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
		reporters: ['default'],
		coverage: {
			reportsDirectory: '../../../coverage/apps/content-scripts/youtube-watcher',
			provider: 'v8',
		},
	},
});
