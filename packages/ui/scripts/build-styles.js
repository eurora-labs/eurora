import { spawn } from 'node:child_process';
import { copyFile, mkdir } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, '..');
const srcStylesDir = join(projectRoot, 'src', 'styles');
const distStylesDir = join(projectRoot, 'dist', 'styles');

/**
 * `main.css` is a Tailwind *source* file (no `@import 'tailwindcss'`); apps
 * pair it with their own Tailwind compile, so we ship it raw to preserve the
 * `@theme inline` overrides intact. `preview.css` is the standalone wrapper
 * for consumers that don't run Tailwind themselves (Storybook, the chat
 * package's dev playground), so we pre-compile it to a self-contained bundle.
 */
const TAILWIND_SOURCE_FILES = ['main.css'];
const PRECOMPILED_FILES = ['preview.css'];

export async function buildStyles() {
	await mkdir(distStylesDir, { recursive: true });

	await Promise.all([
		...TAILWIND_SOURCE_FILES.map((name) =>
			copyFile(join(srcStylesDir, name), join(distStylesDir, name)),
		),
		...PRECOMPILED_FILES.map((name) => compileWithPostcss(name)),
	]);
}

// `node_modules/.bin/postcss` is a shim placed by pnpm; on Windows it's
// `postcss.cmd`. Resolving it directly avoids `shell: true` (and the
// associated argument-escaping deprecation) without losing Windows support.
const postcssBin = join(
	projectRoot,
	'node_modules',
	'.bin',
	process.platform === 'win32' ? 'postcss.cmd' : 'postcss',
);

function compileWithPostcss(name) {
	return new Promise((resolve, reject) => {
		const child = spawn(
			postcssBin,
			[join('./src/styles', name), '-o', join('./dist/styles', name)],
			{ cwd: projectRoot, stdio: 'inherit' },
		);

		child.on('error', reject);
		child.on('close', (code) => {
			if (code === 0) {
				resolve();
			} else {
				reject(new Error(`postcss exited with code ${code} while building ${name}`));
			}
		});
	});
}

const invokedDirectly = import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
	buildStyles().catch((error) => {
		console.error(error);
		process.exit(1);
	});
}
