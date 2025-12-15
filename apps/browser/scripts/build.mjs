import { build } from 'vite';
import { popupConfig } from '../vite/presets/popup.js';
import { contentConfig } from '../vite/presets/content.js';
import { backgroundConfig } from '../vite/presets/background.js';
import { writeManifest } from '../manifest/targets.js';

async function main() {
	const browser = process.env['BROWSER'] ?? 'chrome'; // chrome | firefox | safari
	const outDir = `dist/${browser}`;

	await build(popupConfig({ browser, outDir, emptyOutDir: true }));
	await build(contentConfig({ browser, outDir, emptyOutDir: false }));
	await build(backgroundConfig({ browser, outDir, emptyOutDir: false }));

	await writeManifest({ browser, outDir });
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
