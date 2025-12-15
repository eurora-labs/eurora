import { defineConfig } from 'vite';

export function backgroundConfig({ browser, outDir, emptyOutDir }) {
	const input =
		browser === 'firefox'
			? 'src/background/entry.firefox.ts'
			: 'src/background/entry.chrome.ts';
	return defineConfig({
		define: {
			__BROWSER__: JSON.stringify(browser),
		},
		build: {
			outDir,
			emptyOutDir,
			rollupOptions: {
				input: {
					background: input,
				},
				output: {
					entryFileNames: 'assets/[name].js',
				},
			},
		},
	});
}
