/* eslint-disable no-console, @typescript-eslint/ban-ts-comment */
import { writeManifest } from '../manifest/targets.js';
import { backgroundConfig } from '../vite/presets/background.js';
import { contentConfig } from '../vite/presets/content.js';
import { build } from 'vite';
import { execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

async function main() {
	const browser = process.env['BROWSER'] ?? 'chrome'; // chrome | firefox | safari
	const outDir = `dist/${browser}`;

	// Clean output directory
	fs.rmSync(outDir, { recursive: true, force: true });
	fs.mkdirSync(outDir, { recursive: true });

	// Build SvelteKit popup using command line (it uses vite.config.ts and svelte.config.js)
	console.log('Building SvelteKit popup...');
	execSync('pnpm exec vite build', { stdio: 'inherit' });

	// Copy SvelteKit build output to the target directory
	const sveltekitBuildDir = 'build';
	if (fs.existsSync(sveltekitBuildDir)) {
		copyDir(sveltekitBuildDir, outDir);

		// Rename index.html to popup.html for browser extension
		const indexHtml = path.join(outDir, 'index.html');
		const popupHtml = path.join(outDir, 'popup.html');
		if (fs.existsSync(indexHtml)) {
			fs.renameSync(indexHtml, popupHtml);
		}

		// For Safari: rename script-{hash}.js to script.js and update references
		if (browser === 'safari') {
			const scriptFiles = fs
				.readdirSync(outDir)
				.filter((f) => /^script-[a-z0-9]+\.js$/.test(f));
			for (const scriptFile of scriptFiles) {
				const newName = 'script.js';
				fs.renameSync(path.join(outDir, scriptFile), path.join(outDir, newName));
				console.log(`Renamed ${scriptFile} -> ${newName}`);
				// Update reference in popup.html
				if (fs.existsSync(popupHtml)) {
					let html = fs.readFileSync(popupHtml, 'utf-8');
					html = html.replace(`/${scriptFile}`, `/${newName}`);
					fs.writeFileSync(popupHtml, html);
				}
			}
		}
	}

	// Build content and background scripts
	console.log('Building content script...');
	// @ts-ignore
	await build(contentConfig({ browser, outDir, emptyOutDir: false }));

	console.log('Building background script...');
	// @ts-ignore
	await build(backgroundConfig({ browser, outDir, emptyOutDir: false }));

	await writeManifest({ browser, outDir });
}

function copyDir(src, dest) {
	fs.mkdirSync(dest, { recursive: true });
	const entries = fs.readdirSync(src, { withFileTypes: true });
	for (const entry of entries) {
		const srcPath = path.join(src, entry.name);
		const destPath = path.join(dest, entry.name);
		if (entry.isDirectory()) {
			copyDir(srcPath, destPath);
		} else {
			fs.copyFileSync(srcPath, destPath);
		}
	}
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
