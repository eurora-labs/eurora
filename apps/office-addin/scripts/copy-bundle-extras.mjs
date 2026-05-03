#!/usr/bin/env node
// Copies build artifacts that vite doesn't own into dist/ so the directory is
// fully self-contained for the Tauri installer. Vite handles HTML/JS/assets and
// public/ (icons); this script handles the manifest template, which is consumed
// by the desktop at runtime to render manifest.xml.

import { copyFile, mkdir } from 'node:fs/promises';
import { dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = new URL('../', import.meta.url);

const COPIES = [['manifest.template.xml', 'dist/manifest.template.xml']];

for (const [src, dst] of COPIES) {
	const from = new URL(src, projectRoot);
	const to = new URL(dst, projectRoot);
	await mkdir(dirname(fileURLToPath(to)), { recursive: true });
	await copyFile(from, to);
	process.stdout.write(`copied ${relative(process.cwd(), fileURLToPath(to))}\n`);
}
