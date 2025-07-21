import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import devtoolsJson from 'vite-plugin-devtools-json';

export default defineConfig({
	// server: {
	// 	port: 3000,
	// },
	plugins: [sveltekit(), devtoolsJson()],
});
