// vite.config.ts
import { defineConfig, loadEnv } from 'vite';
import { popupConfig } from './vite/presets/popup.js';
import { contentConfig } from './vite/presets/content.js';
import { backgroundConfig } from './vite/presets/background.js';

export default defineConfig(({ command, mode }) => {
	const env = loadEnv(mode, process.cwd(), '');
	const browser = env['BROWSER'] || process.env['BROWSER'] || 'chrome';
	const entry = env['ENTRY'] || process.env['ENTRY'] || (command === 'serve' ? 'popup' : 'popup');

	const outDir = env['OUT_DIR'] || process.env['OUT_DIR'] || `dist/${browser}`;

	// This file is for:
	// - `vite dev` (defaults to popup)
	// - `ENTRY=content vite build`, etc.
	// Multi-step builds should use scripts/build.mjs (vite.build() x3).
	if (entry === 'content') return contentConfig({ browser, outDir, emptyOutDir: true });
	if (entry === 'background') return backgroundConfig({ browser, outDir, emptyOutDir: true });
	return popupConfig({ browser, outDir, emptyOutDir: true });
});
