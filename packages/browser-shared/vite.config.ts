import { defineConfig } from 'vitest/config';
import dts from 'vite-plugin-dts';
import path from 'path';

export default defineConfig({
	plugins: [
		dts({
			entryRoot: 'src',
			outDir: 'dist',
			tsconfigPath: path.join(__dirname, 'tsconfig.json'),
		}),
	],
	test: {
		include: ['src/**/*.(test|spec).?(m)[jt]s?(x)'],
	},
	build: {
		outDir: 'dist',
		emptyOutDir: true,
		reportCompressedSize: true,
		sourcemap: true,

		lib: {
			entry: path.resolve(__dirname, 'src/index.ts'),
			formats: ['es'],
		},

		rollupOptions: {
			external: [],
			output: {
				format: 'es',
				preserveModules: true,
				preserveModulesRoot: 'src',
				entryFileNames: '[name].js',
				chunkFileNames: 'chunks/[name]-[hash].js',
				assetFileNames: 'assets/[name].[ext]',
			},
		},
	},
});
