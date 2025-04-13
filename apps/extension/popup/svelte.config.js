// import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import adapter from 'sveltekit-adapter-chrome-extension';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url); // get the resolved path to the file
const __dirname = path.dirname(__filename); // get the name of the directory

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	preprocess: vitePreprocess({script: true}),

	kit: {
        adapter: adapter({
            
			pages: path.join(__dirname, '../../../extensions/chromium/pages/popup'),
			assets: path.join(__dirname, '../../../extensions/chromium/pages/popup'),
			// pages: 'build',
			// assets: 'build',
			// fallback: 'index.html',
			fallback: null,
			precompress: false,
            strict: false,
            // manifest: 'manifest.json'
            
        }),
        appDir: 'app',
        paths: {
            relative: false,
        },
        csp: {
            directives: {
                'script-src': ['unsafe-inline', 'unsafe-eval', 'self'],
            },
            reportOnly: {
                'script-src': ['self'],
                'report-uri': ['/']
            },
            mode: 'auto'
        },
	},
	compilerOptions: {
		css: 'injected',
		enableSourcemap: true
    },
    // paths: {
    //     base: "./"
    // }
};

export default config;
