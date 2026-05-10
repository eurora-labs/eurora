import { existsSync, watch } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { buildStyles } from './build-styles.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, '..');
const srcStylesDir = join(projectRoot, 'src', 'styles');
const distStylesFile = join(projectRoot, 'dist', 'styles', 'main.css');

let isBuilding = false;
let buildQueued = false;

async function runBuild() {
	if (isBuilding) {
		buildQueued = true;
		return;
	}

	isBuilding = true;
	console.log('[watch-styles] Building styles...');

	try {
		await buildStyles();
		console.log('[watch-styles] Styles built successfully');
	} catch (error) {
		console.error('[watch-styles]', error);
	} finally {
		isBuilding = false;
		if (buildQueued) {
			buildQueued = false;
			runBuild();
		}
	}
}

await runBuild();

console.log('[watch-styles] Watching src/styles/ for changes...');
watch(srcStylesDir, { recursive: true }, (_eventType, filename) => {
	if (filename && filename.endsWith('.css')) {
		console.log(`[watch-styles] Detected change in ${filename}`);
		runBuild();
	}
});

// `svelte-package --watch` wipes `dist/` whenever it republishes the package,
// which races our copy and would otherwise leave consumers without a CSS
// bundle until the next manual change. Poll for the marker file and rebuild
// on its absence so the sibling Svelte build can't silently win the race.
console.log('[watch-styles] Watching dist/styles/...');
setInterval(() => {
	if (!existsSync(distStylesFile)) {
		console.log('[watch-styles] Styles file missing, rebuilding...');
		runBuild();
	}
}, 1000);

console.log('[watch-styles] Style watcher started');
