import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import path from 'path';
import tailwindcss from '@tailwindcss/vite';

console.log(path.resolve(__dirname, '../../packages/ui/src'));

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	resolve: {
		alias: {
			'@eurora/katex': path.resolve(
				__dirname,
				'../../packages/custom-components/katex/src/lib/index.ts'
			)
		}
	}
});
