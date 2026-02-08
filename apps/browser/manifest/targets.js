import { base } from './base.js';
import fs from 'node:fs';
import path from 'node:path';

function deepMerge(a, b) {
	if (Array.isArray(a) || Array.isArray(b)) return b ?? a;
	if (a && typeof a === 'object' && b && typeof b === 'object') {
		const out = { ...a };
		for (const k of Object.keys(b)) out[k] = deepMerge(a[k], b[k]);
		return out;
	}
	return b ?? a;
}

// Firefox extension ID configuration
// Development ID is used by default, production ID should be set via FIREFOX_EXTENSION_ID env var
const FIREFOX_DEV_ID = 'dev@eurora-labs.com';
const _FIREFOX_PROD_ID = '{271903fe-1905-4636-b47f-6f0873dc97f8}';

function targetPatch(browser) {
	if (browser === 'firefox') {
		// Use FIREFOX_EXTENSION_ID env var if set, otherwise use dev ID
		const firefoxId = process.env['FIREFOX_EXTENSION_ID'] || FIREFOX_DEV_ID;
		return {
			browser_specific_settings: {
				gecko: { id: firefoxId },
			},
			background: { scripts: ['assets/background.js'] },
			// Firefox MV3 differences frequently land here
		};
	}
	if (browser === 'safari') {
		return {
			background: { scripts: ['assets/background.js'] },
		};
	}
	return {
		background: { service_worker: 'assets/background.js', type: 'module' },
	}; // chrome baseline
}

export async function writeManifest({ browser, outDir }) {
	const manifest = deepMerge(base, targetPatch(browser));
	// Use VERSION env var if set, otherwise default to 0.0.0
	manifest.version = process.env['VERSION'] || '0.0.0';

	fs.mkdirSync(outDir, { recursive: true });
	fs.writeFileSync(path.join(outDir, 'manifest.json'), JSON.stringify(manifest, null, 2), 'utf8');
}
