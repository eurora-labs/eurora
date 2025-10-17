import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { readdirSync, cpSync } from 'fs';
import path from 'path';

const chromiumOutDir = path.resolve(__dirname, '../../../extensions/chromium/pages/popup');
const firefoxOutDir = path.resolve(__dirname, '../../../extensions/firefox/pages/popup');

function CopyToFirefoxPlugin() {
	return {
		name: 'copy-to-firefox',
		closeBundle() {
			// Copy all files from chromium output to firefox output
			cpSync(chromiumOutDir, firefoxOutDir, { recursive: true });
			console.log(`✓ Copied content scripts to firefox extension folder`);
		},
	};
}

export default defineConfig({
	plugins: [sveltekit(), CopyToFirefoxPlugin()],
	build: {
		outDir: chromiumOutDir,
	},
});
