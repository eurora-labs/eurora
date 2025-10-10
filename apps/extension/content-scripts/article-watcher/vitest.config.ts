/// <reference types='vitest' />
import { defineConfig } from 'vitest/config';
import * as path from 'path';

export default defineConfig({
	test: {
		globals: true,
		environment: 'jsdom',
		setupFiles: ['./vitest-setup.ts'],
		include: ['src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
		coverage: {
			provider: 'v8',
			reporter: ['text', 'json', 'html'],
			exclude: ['node_modules/', 'dist/', '**/*.d.ts', '**/*.config.*', '**/mockData'],
		},
	},
	resolve: {
		alias: {
			'@eurora/chrome-ext-shared': path.resolve(
				__dirname,
				'../../../../packages/chrome-ext-shared/src/lib',
			),
		},
	},
});
