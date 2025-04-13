import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
    plugins: [sveltekit()],
    build: {
        outDir: path.join(__dirname, '../../../extensions/chromium/pages/popup'),
        emptyOutDir: true,
        reportCompressedSize: true,
        commonjsOptions: {
            transformMixedEsModules: true
        },
        cssCodeSplit: true,
    },
	resolve: {
		alias: {
			'@eurora/ui': path.resolve(__dirname, '../../../packages/ui/src'),
			'@eurora/katex': path.resolve(
				__dirname,
				'../../../packages/custom-components/katex/src/lib/index.ts'
			)
		}
	}
});
