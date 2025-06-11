import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [sveltekit()],
	test: {
		include: ['src/**/*.{test,spec}.{js,ts,mjs,mts,jsx,tsx}'],
		environment: 'jsdom',
	},
	build: {
		sourcemap: 'inline',
	},
});
