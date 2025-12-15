import fs from 'node:fs';
import path from 'node:path';
import { base } from './base.js';

function deepMerge(a, b) {
	if (Array.isArray(a) || Array.isArray(b)) return b ?? a;
	if (a && typeof a === 'object' && b && typeof b === 'object') {
		const out = { ...a };
		for (const k of Object.keys(b)) out[k] = deepMerge(a[k], b[k]);
		return out;
	}
	return b ?? a;
}

function targetPatch(browser) {
	if (browser === 'firefox') {
		return {
			browser_specific_settings: {
				gecko: { id: 'your-id@eurora-labs.com' },
			},
			// Firefox MV3 differences frequently land here
		};
	}
	if (browser === 'safari') {
		return {
			// keep it WebExtensions-compatible; Safari packaging is done after build
		};
	}
	return {}; // chrome baseline
}

export async function writeManifest({ browser, outDir }) {
	const manifest = deepMerge(base, targetPatch(browser));
	manifest.version = '0.0.0';

	fs.mkdirSync(outDir, { recursive: true });
	fs.writeFileSync(path.join(outDir, 'manifest.json'), JSON.stringify(manifest, null, 2), 'utf8');
}
