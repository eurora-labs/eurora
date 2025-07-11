import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import copy from 'rollup-plugin-copy';

// import copyBuild from '@eurora/shared/util/copy-plugin';

export default defineConfig({
	plugins: [
		sveltekit(),
		// copyBuild([
		// 	'../../../extensions/chromium/pages/popup',
		// 	'../../../extensions/firefox/pages/popup',
		// ]),
	],
	build: {
		rollupOptions: {
			plugins: [
				copy({
					targets: [
						{ src: 'build/**/*', dest: '../../../extensions/chromium/pages/popup' },
						{ src: 'build/**/*', dest: '../../../extensions/firefox/pages/popup' },
					],
					hook: 'closeBundle', // run after Vite writes output
					overwrite: true,
				}),
			],
		},
	},
});
