/* eslint-disable no-console, @typescript-eslint/no-unused-vars, @typescript-eslint/ban-ts-comment */
import { writeManifest } from '../manifest/targets.js';
import { backgroundConfig } from '../vite/presets/background.js';
import { contentConfig } from '../vite/presets/content.js';
import { build } from 'vite';
import { execSync } from 'node:child_process';
import fs, { createWriteStream } from 'node:fs';
import path from 'node:path';
import { pipeline } from 'node:stream/promises';

const PDFJS_CACHE_DIR = '.pdfjs-viewer';
const PDFJS_TEMP_DIR = '.pdfjs-temp';
const VERSION_FILE = path.join(PDFJS_CACHE_DIR, '.version');
const PDFJS_VERSION = '5.4.551';

async function main() {
	const browser = process.env['BROWSER'] ?? 'chrome'; // chrome | firefox | safari
	const outDir = `dist/${browser}`;

	// Clean output directory
	fs.rmSync(outDir, { recursive: true, force: true });
	fs.mkdirSync(outDir, { recursive: true });

	// Ensure PDF.js viewer cache exists (Chrome only)
	if (browser === 'chrome') {
		await ensurePdfjsCache();
	}

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

	// Copy preferences schema to dist folder (next to manifest.json)
	const preferencesSchemaPath = path.join('src', 'preferences_schema.json');
	if (fs.existsSync(preferencesSchemaPath)) {
		fs.copyFileSync(preferencesSchemaPath, path.join(outDir, 'preferences_schema.json'));
		console.log('Copied preferences_schema.json to', outDir);
	}

	// Copy PDF.js viewer files (Chrome only)
	if (browser === 'chrome') {
		await copyPdfjsViewer(outDir);
	}
}

// ============================================================================
// PDF.js Setup Functions
// ============================================================================

/**
 * Check if the cached version matches the required version
 */
function isCacheValid(requiredVersion) {
	if (!fs.existsSync(VERSION_FILE)) {
		return false;
	}

	const cachedVersion = fs.readFileSync(VERSION_FILE, 'utf-8').trim();
	return cachedVersion === requiredVersion;
}

/**
 * Download a file from URL
 */
async function downloadFile(url, destPath) {
	console.log(`Downloading: ${url}`);

	const response = await fetch(url, {
		redirect: 'follow',
	});

	if (!response.ok) {
		throw new Error(`Failed to download ${url}: ${response.status} ${response.statusText}`);
	}

	fs.mkdirSync(path.dirname(destPath), { recursive: true });

	const fileStream = createWriteStream(destPath);
	// @ts-ignore
	await pipeline(response.body, fileStream);
}

/**
 * Extract a ZIP file using system unzip command
 */
async function extractZip(zipPath, destDir) {
	const { promisify } = await import('node:util');
	const execAsync = promisify(execSync.constructor);

	fs.mkdirSync(destDir, { recursive: true });

	// Use system unzip command (available on most systems)
	execSync(`unzip -o -q "${zipPath}" -d "${destDir}"`, { stdio: 'inherit' });
}

/**
 * Copy relevant files from extracted pdfjs to cache directory
 */
function setupPdfjsCache(extractedDir, version) {
	// List contents of extracted directory for debugging
	const extractedContents = fs.readdirSync(extractedDir);
	console.log('Extracted contents:', extractedContents);

	// The zip might extract to a subdirectory or directly to the extraction dir
	// Look for: pdfjs-X.Y.Z-dist, pdfjs-dist, or web/ and build/ directly
	let sourceDir = extractedDir;

	// Check if there's a pdfjs-* subdirectory
	const pdfjsDir = extractedContents.find(
		(name) =>
			name.startsWith('pdfjs-') && fs.statSync(path.join(extractedDir, name)).isDirectory(),
	);

	if (pdfjsDir) {
		sourceDir = path.join(extractedDir, pdfjsDir);
	} else if (extractedContents.includes('content')) {
		// Files extracted directly to the extraction directory
		sourceDir = path.join(extractedDir, 'content');
	} else {
		console.error('Extracted directory contents:', extractedContents);
		throw new Error(
			'Could not find extracted pdfjs directory structure (expected pdfjs-* folder or web/ and build/ directories)',
		);
	}

	console.log('Using source directory:', sourceDir);

	// Clean the cache directory
	if (fs.existsSync(PDFJS_CACHE_DIR)) {
		fs.rmSync(PDFJS_CACHE_DIR, { recursive: true });
	}
	fs.mkdirSync(PDFJS_CACHE_DIR, { recursive: true });

	// Copy web directory (viewer.html, viewer.mjs, viewer.css, locale, images)
	const webSourceDir = path.join(sourceDir, 'web');
	const webDestDir = path.join(PDFJS_CACHE_DIR, 'web');
	copyDir(webSourceDir, webDestDir);

	// Copy build directory (pdf.mjs, pdf.worker.mjs, etc.)
	const buildSourceDir = path.join(sourceDir, 'build');
	const buildDestDir = path.join(PDFJS_CACHE_DIR, 'build');
	copyDir(buildSourceDir, buildDestDir);

	// Write version file
	fs.writeFileSync(VERSION_FILE, version);

	console.log(`PDF.js viewer v${version} cached successfully`);
}

/**
 * Ensure PDF.js viewer cache exists by downloading if needed
 */
async function ensurePdfjsCache() {
	console.log(`Required pdfjs-dist version: ${PDFJS_VERSION}`);

	// Check if cache is valid
	if (isCacheValid(PDFJS_VERSION)) {
		console.log(`PDF.js viewer v${PDFJS_VERSION} already cached`);
		return;
	}

	console.log(`Downloading PDF.js viewer v${PDFJS_VERSION}...`);

	// Mozilla's official PDF.js releases on GitHub
	const releaseUrl = `https://github.com/eurora-labs/pdf.js/releases/download/chromium-v${PDFJS_VERSION}/pdfjs-${PDFJS_VERSION}-chromium.zip`;

	const zipPath = path.join(PDFJS_TEMP_DIR, 'pdfjs.zip');

	try {
		// Download the release
		await downloadFile(releaseUrl, zipPath);

		// Extract the ZIP
		console.log('Extracting PDF.js...');
		await extractZip(zipPath, PDFJS_TEMP_DIR);

		// Copy relevant files to cache
		setupPdfjsCache(PDFJS_TEMP_DIR, PDFJS_VERSION);
	} finally {
		// Clean up temp directory
		if (fs.existsSync(PDFJS_TEMP_DIR)) {
			fs.rmSync(PDFJS_TEMP_DIR, { recursive: true });
		}
	}
}

/**
 * Copy PDF.js viewer files to the output directory
 *
 * Structure:
 * - content/build/  -> pdf.mjs, pdf.worker.mjs, pdf.sandbox.mjs
 * - content/web/    -> viewer.html (custom), viewer.mjs, viewer.css, locale/, images/
 */
async function copyPdfjsViewer(outDir) {
	console.log('Copying PDF.js viewer files...');

	const contentDir = path.join(outDir, 'content');
	const buildDir = path.join(contentDir, 'build');
	const webDir = path.join(contentDir, 'web');

	// Create directories
	fs.mkdirSync(buildDir, { recursive: true });
	fs.mkdirSync(webDir, { recursive: true });

	// Copy build files (pdf.mjs, pdf.worker.mjs, etc.)
	const cacheBuildDir = path.join(PDFJS_CACHE_DIR, 'build');
	if (fs.existsSync(cacheBuildDir)) {
		copyDir(cacheBuildDir, buildDir);
		console.log('  - Copied PDF.js build files');
	}

	// Copy web files (viewer.mjs, viewer.css, locale/, images/)
	const cacheWebDir = path.join(PDFJS_CACHE_DIR, 'web');
	if (fs.existsSync(cacheWebDir)) {
		copyDir(cacheWebDir, webDir);
		console.log('  - Copied PDF.js web files');
	}

	// Copy custom viewer.html from src (overrides the one from cache)
	// This viewer.html includes the custom _pdf handler script
	const customViewerHtml = path.join('src', 'viewer.html');
	const destViewerHtml = path.join(webDir, 'viewer.html');
	if (fs.existsSync(customViewerHtml)) {
		fs.copyFileSync(customViewerHtml, destViewerHtml);
		console.log('  - Copied custom viewer.html with _pdf handler');
	}

	// Inject PDF handler script into viewer.html
	injectPdfHandlerScript(destViewerHtml);

	console.log('PDF.js viewer files copied successfully');
}

/**
 * Inject the PDF handler script into viewer.html
 * Adds a script tag just above the closing </head> tag
 */
function injectPdfHandlerScript(viewerHtmlPath) {
	if (!fs.existsSync(viewerHtmlPath)) {
		console.warn('  - viewer.html not found, skipping script injection');
		return;
	}

	let content = fs.readFileSync(viewerHtmlPath, 'utf-8');
	const scriptTag =
		'<script src="../../scripts/content/sites/_pdf/index.js" type="module"></script>';
	const closingHeadTag = '</head>';

	if (content.includes(scriptTag)) {
		console.log('  - PDF handler script already present in viewer.html');
		return;
	}

	if (!content.includes(closingHeadTag)) {
		console.warn('  - Could not find </head> tag in viewer.html, skipping script injection');
		return;
	}

	// Insert the script tag just above the closing </head> tag
	content = content.replace(closingHeadTag, `${scriptTag}\n${closingHeadTag}`);
	fs.writeFileSync(viewerHtmlPath, content);
	console.log('  - Injected PDF handler script into viewer.html');
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
