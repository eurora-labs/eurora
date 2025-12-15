import { defineConfig } from 'vite';

export function contentConfig({ browser, outDir, emptyOutDir }) {
	return defineConfig({
		define: {
			__BROWSER__: JSON.stringify(browser),
		},
		plugins: [
			// your URL-based “code injection per website” plugins live here
		],
		build: {
			outDir,
			emptyOutDir,
			rollupOptions: {
				input: {
					content: 'src/content/index.ts',
					// optional: emit per-site injection files as separate inputs
					// inject_google: 'src/content/inject/google.ts',
				},
				output: {
					entryFileNames: 'assets/[name].js',
					chunkFileNames: 'assets/[name]-[hash].js',
					assetFileNames: 'assets/[name]-[hash][extname]',
				},
			},
		},
	});
}
