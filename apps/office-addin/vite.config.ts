import devCerts from 'office-addin-dev-certs';
import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

const DEV_PORT = 3000;
const projectRoot = fileURLToPath(new URL('.', import.meta.url));
const runtimeRoot = fileURLToPath(new URL('./src/runtime', import.meta.url));
const srcRoot = fileURLToPath(new URL('./src', import.meta.url));

export default defineConfig(async ({ command }) => {
	const httpsOptions =
		command === 'serve'
			? await devCerts.getHttpsServerOptions().catch(() => undefined)
			: undefined;

	return {
		root: runtimeRoot,
		base: './',
		resolve: {
			alias: {
				$lib: srcRoot,
			},
		},
		build: {
			outDir: fileURLToPath(new URL('./dist', import.meta.url)),
			emptyOutDir: true,
			sourcemap: true,
			target: 'es2022',
			rollupOptions: {
				input: fileURLToPath(new URL('./src/runtime/runtime.html', import.meta.url)),
				output: {
					entryFileNames: 'assets/[name]-[hash].js',
					chunkFileNames: 'assets/[name]-[hash].js',
					assetFileNames: 'assets/[name]-[hash][extname]',
				},
			},
		},
		server: {
			port: DEV_PORT,
			strictPort: true,
			host: 'localhost',
			https: httpsOptions,
		},
		preview: {
			port: DEV_PORT,
			strictPort: true,
			host: 'localhost',
			https: httpsOptions,
		},
		test: {
			root: projectRoot,
			environment: 'jsdom',
			include: ['src/**/*.test.ts'],
			globals: false,
		},
	};
});
