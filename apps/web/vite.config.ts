import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import path from 'path';

console.log(path.resolve(__dirname, '../../packages/ui/src'));

export default defineConfig({
	plugins: [sveltekit()],
	resolve: {
		alias: {
			'@eurora/ui': path.resolve(__dirname, '../../packages/ui/src'),
			'@eurora/katex': path.resolve(
				__dirname,
				'../../packages/custom-components/katex/src/lib/index.ts'
			)
		}
	}
});
