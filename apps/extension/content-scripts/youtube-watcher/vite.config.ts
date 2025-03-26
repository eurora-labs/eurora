/// <reference types='vitest' />
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';
import * as path from 'path';

export default defineConfig({
	root: __dirname,
	plugins: [
		dts({
			entryRoot: 'src',
			tsconfigPath: path.join(__dirname, 'tsconfig.json')
		})
	],
	// Uncomment this if you are using workers.
	// worker: {
	//  plugins: [ nxViteTsPaths() ],
	// },
	// Configuration for building your library.
	// See: https://vitejs.dev/guide/build.html#library-mode
	build: {
		// outDir: '../../../dist/apps/content-scripts/youtube-watcher',
		outDir: path.resolve(
			__dirname,
			'../../../../extensions/chromium/content-scripts/youtube-watcher'
		),
		emptyOutDir: true,
		reportCompressedSize: true,
		commonjsOptions: {
			transformMixedEsModules: true
		},
		lib: {
			// Could also be a dictionary or array of multiple entry points.
			entry: 'src/index.ts',
			name: 'youtube-watcher',
			fileName: 'index',
			// Change this to the formats you want to support.
			// Don't forget to update your package.json as well.
			formats: ['es']
		},
		rollupOptions: {
			// External packages that should not be bundled into your library.
			external: [],
			output: {
				entryFileNames: 'main.js',
				chunkFileNames: 'main-[name].js',
				assetFileNames: 'assets/[name].[ext]'
			}
		}
	},
	test: {
		watch: false,
		globals: true,
		environment: 'node',
		include: ['src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
		reporters: ['default'],
		coverage: {
			reportsDirectory: '../../../coverage/apps/content-scripts/youtube-watcher',
			provider: 'v8'
		}
	},
	resolve: {
		alias: {
			'@eurora/youtube-transcripts': path.resolve(
				__dirname,
				'../../../../packages/website-modules/youtube/youtube-transcripts/src/index.ts'
			),
			'@eurora/proto/*': path.resolve(__dirname, '../../../../packages/proto/src/lib/*')
		}
	}
});
