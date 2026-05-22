import path from 'node:path';

// Background entry is the same module for every target. Browser-specific
// manifest differences (Firefox `scripts: [...]` vs. Chrome `service_worker`)
// are applied by `manifest/targets.js`; the bundled JS is identical.
export function backgroundConfig({ browser, outDir, emptyOutDir, mode }) {
	const rootDir = path.resolve(import.meta.dirname, '../..');
	return {
		// Don't load vite.config.ts (which has SvelteKit) for this build
		configFile: false,
		mode: mode || 'production',
		root: rootDir,
		define: {
			__BROWSER__: JSON.stringify(browser),
			__DEV__: JSON.stringify(mode === 'development'),
		},
		build: {
			outDir,
			emptyOutDir,
			rollupOptions: {
				input: {
					background: path.resolve(rootDir, 'src/background/index.ts'),
				},
				output: {
					format: 'es',
					entryFileNames: 'assets/[name].js',
				},
			},
			target: 'es2022',
			minify: 'esbuild',
			modulePreload: false,
			sourcemap: false,
		},
	};
}
