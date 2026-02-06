import path from 'node:path';

export function backgroundConfig({ browser, outDir, emptyOutDir }) {
	const rootDir = path.resolve(import.meta.dirname, '../..');
	const input =
		browser === 'firefox' || browser === 'safari'
			? 'src/background/entry.firefox.ts'
			: 'src/background/entry.chrome.ts';
	return {
		// Don't load vite.config.ts (which has SvelteKit) for this build
		configFile: false,
		root: rootDir,
		define: {
			__BROWSER__: JSON.stringify(browser),
		},
		build: {
			outDir,
			emptyOutDir,
			rollupOptions: {
				input: {
					background: path.resolve(rootDir, input),
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
