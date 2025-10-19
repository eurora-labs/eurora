/// <reference types='vitest' />
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';
import path from 'path';

export default defineConfig({
	root: __dirname,
	plugins: [
		dts({
			entryRoot: 'src',
			tsconfigPath: path.join(__dirname, 'tsconfig.json'),
		}),
		// sentryVitePlugin({
		// 	org: 'eurora-labs',
		// 	project: 'apps_extension_background-script'
		// })
	],
	// Uncomment this if you are using workers.
	// worker: {
	//  plugins: [ nxViteTsPaths() ],
	// },
	// Configuration for building your library.
	// See: https://vitejs.dev/guide/build.html#library-mode
	build: {
		outDir: path.resolve(__dirname, '../../../../extensions/chromium/scripts/background'),
		emptyOutDir: true,
		reportCompressedSize: true,

		commonjsOptions: {
			transformMixedEsModules: true,
		},

		lib: {
			// Could also be a dictionary or array of multiple entry points.
			entry: 'src/index.ts',
			name: 'background-script',
			fileName: 'index',
			// Change this to the formats you want to support.
			// Don't forget to update your package.json as well.
			formats: ['es'],
		},

		rollupOptions: {
			// External packages that should not be bundled into your library.
			external: [],
			input: {
				main: 'src/index.ts',
				// 'service-worker/messaging-worker': 'src/lib/service-worker/messaging-worker.ts'
			},
			output: {
				entryFileNames: '[name].js',
				chunkFileNames: 'chunks/[name]-[hash].js',
				assetFileNames: 'assets/[name].[ext]',
			},
		},

		sourcemap: true,
	},
	// test: {
	// 	watch: false,
	// 	globals: true,
	// 	environment: 'node',
	// 	include: ['src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
	// 	reporters: ['default'],
	// 	coverage: {
	// 		reportsDirectory: '../../../coverage/apps/extension/background-script',
	// 		provider: 'v8'
	// 	}
	// },
});
