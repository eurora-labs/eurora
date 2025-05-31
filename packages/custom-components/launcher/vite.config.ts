import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
	plugins: [sveltekit(), tailwindcss()],
	test: {
		include: ['src/**/*.(test|spec).?(m)[jt]s?(x)'],
		environment: 'jsdom'
	},
	build: {
		sourcemap: 'inline'
	}
});
