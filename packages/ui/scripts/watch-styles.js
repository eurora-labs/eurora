import { spawn } from 'child_process';
import { watch } from 'fs';
import { existsSync, mkdirSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, '..');
const distStylesDir = join(projectRoot, 'dist', 'styles');
const distStylesFile = join(distStylesDir, 'main.css');

let isBuilding = false;
let buildQueued = false;

function buildStyles() {
	if (isBuilding) {
		buildQueued = true;
		return;
	}

	isBuilding = true;
	// eslint-disable-next-line no-console
	console.log('[watch-styles] Building styles...');

	const build = spawn(
		'pnpm',
		['exec', 'postcss', './src/styles/main.css', '-o', './dist/styles/main.css'],
		{
			cwd: projectRoot,
			stdio: 'inherit',
			shell: true,
		},
	);

	build.on('close', (code) => {
		isBuilding = false;
		if (code === 0) {
			// eslint-disable-next-line no-console
			console.log('[watch-styles] Styles built successfully');
		} else {
			console.error(`[watch-styles] Build failed with code ${code}`);
		}

		if (buildQueued) {
			buildQueued = false;
			buildStyles();
		}
	});
}

// Initial build
buildStyles();

// Watch source files
// eslint-disable-next-line no-console
console.log('[watch-styles] Watching src/styles/ for changes...');
watch(join(projectRoot, 'src', 'styles'), { recursive: true }, (eventType, filename) => {
	if (filename && filename.endsWith('.css')) {
		// eslint-disable-next-line no-console
		console.log(`[watch-styles] Detected change in ${filename}`);
		buildStyles();
	}
});

// Watch dist directory for deletion/recreation
// eslint-disable-next-line no-console
console.log('[watch-styles] Watching dist/styles/...');

// Check periodically if the styles file exists, rebuild if missing
setInterval(() => {
	if (!existsSync(distStylesFile)) {
		// eslint-disable-next-line no-console
		console.log('[watch-styles] Styles file missing, rebuilding...');
		// Ensure directory exists
		if (!existsSync(distStylesDir)) {
			mkdirSync(distStylesDir, { recursive: true });
		}
		buildStyles();
	}
}, 1000); // Check every second

// eslint-disable-next-line no-console
console.log('[watch-styles] Style watcher started');
