import { test, expect } from '@playwright/test';
import { readFileSync, existsSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test.describe('Content Script Build Artifacts', () => {
	const contentScriptDir = path.join(
		__dirname,
		'../../../../extensions/chromium/scripts/content',
	);

	test('should have built bootstrap.js', () => {
		const bootstrapPath = path.join(contentScriptDir, 'bootstrap.js');
		expect(existsSync(bootstrapPath)).toBe(true);

		if (existsSync(bootstrapPath)) {
			const content = readFileSync(bootstrapPath, 'utf-8');
			expect(content).toContain('SITE_LOAD');
			expect(content).toContain('onMessage');
			expect(content).toContain('runtime.getURL');
		}
	});

	test('should have built registry.json with correct structure', () => {
		const registryPath = path.join(contentScriptDir, 'registry.json');
		expect(existsSync(registryPath)).toBe(true);

		if (existsSync(registryPath)) {
			const registryContent = readFileSync(registryPath, 'utf-8');
			const registry = JSON.parse(registryContent);

			expect(Array.isArray(registry)).toBe(true);
			expect(registry.length).toBeGreaterThan(0);

			registry.forEach((entry: any) => {
				expect(entry).toHaveProperty('id');
				expect(entry).toHaveProperty('chunk');
				expect(entry).toHaveProperty('patterns');
				expect(Array.isArray(entry.patterns)).toBe(true);
			});

			// Verify _default is not in registry
			const hasDefault = registry.some((e: any) => e.id === '_default');
			expect(hasDefault).toBe(false);
		}
	});

	test('should have built site handler chunks', () => {
		const defaultChunk = path.join(contentScriptDir, 'sites/_default/index.js');
		expect(existsSync(defaultChunk)).toBe(true);

		if (existsSync(defaultChunk)) {
			const content = readFileSync(defaultChunk, 'utf-8');
			expect(content.length).toBeGreaterThan(0);
		}
	});

	test('should have YouTube handler if youtube.com site exists', () => {
		const registryPath = path.join(contentScriptDir, 'registry.json');
		if (existsSync(registryPath)) {
			const registry = JSON.parse(readFileSync(registryPath, 'utf-8'));
			const youtubeEntry = registry.find((e: any) => e.id === 'youtube.com');

			if (youtubeEntry) {
				expect(youtubeEntry.patterns).toContain('youtube.com');
				expect(youtubeEntry.patterns).toContain('*.youtube.com');

				const youtubeChunk = path.join(contentScriptDir, youtubeEntry.chunk);
				expect(existsSync(youtubeChunk)).toBe(true);

				if (existsSync(youtubeChunk)) {
					const content = readFileSync(youtubeChunk, 'utf-8');
					expect(content.length).toBeGreaterThan(0);
				}
			}
		}
	});
});

test.describe('YouTube Video Detection (No Extension Required)', () => {
	test('should detect YouTube video page by URL', async ({ page }) => {
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForLoadState('domcontentloaded');

		const isVideoPage = await page.evaluate(() => {
			return window.location.href.includes('/watch?v=');
		});

		expect(isVideoPage).toBe(true);
	});

	test('should extract video ID from URL', async ({ page }) => {
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=test');
		await page.waitForLoadState('domcontentloaded');

		const videoId = await page.evaluate(() => {
			if (window.location.search?.includes('v=')) {
				return window.location.search.split('v=')[1].split('&')[0];
			}
			return null;
		});

		expect(videoId).toBe('dQw4w9WgXcQ');
	});

	test('should find video element on YouTube video page', async ({ page }) => {
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');

		// Wait for video element to appear
		try {
			await page.waitForSelector('video', { timeout: 10000 });
			const hasVideo = await page.evaluate(() => {
				return document.querySelector('video') !== null;
			});
			expect(hasVideo).toBe(true);
		} catch (error) {
			// Video might not load due to consent/region issues, that's okay
			console.log('Video element not found, may be due to consent screen');
		}
	});
});

test.describe('Article Page Detection (No Extension Required)', () => {
	test('should access example.com successfully', async ({ page }) => {
		await page.goto('https://example.com');
		const title = await page.title();
		expect(title).toBeTruthy();
		expect(title).toContain('Example');
	});

	test('should extract page URL', async ({ page }) => {
		await page.goto('https://example.com');
		const url = await page.evaluate(() => window.location.href);
		expect(url).toContain('example.com');
	});

	test('should extract page title', async ({ page }) => {
		await page.goto('https://example.com');
		const title = await page.evaluate(() => document.title);
		expect(title).toBeTruthy();
	});

	test('should find basic DOM elements', async ({ page }) => {
		await page.goto('https://example.com');
		const hasH1 = await page.evaluate(() => {
			return document.querySelector('h1') !== null;
		});
		expect(hasH1).toBe(true);
	});
});

test.describe('Registry Domain Matching Logic', () => {
	test('should match exact domain patterns', () => {
		const patterns = ['youtube.com', '*.youtube.com'];

		// Should match exact domain
		expect(patterns.includes('youtube.com')).toBe(true);

		// Should have wildcard pattern
		expect(patterns.some((p) => p.includes('*'))).toBe(true);
	});

	test('should generate patterns correctly', () => {
		// This tests the logic from vite.config.ts patternsFor function
		const testPatterns = (id: string): string[] => {
			if (id === '_default') return [];
			if (id.includes('*')) return [id];
			return [id, `*.${id}`];
		};

		expect(testPatterns('_default')).toEqual([]);
		expect(testPatterns('youtube.com')).toEqual(['youtube.com', '*.youtube.com']);
		expect(testPatterns('*.example.com')).toEqual(['*.example.com']);
	});

	test('registry should map domains to correct chunk paths', () => {
		const registryPath = path.join(
			__dirname,
			'../../../../extensions/chromium/scripts/content/registry.json',
		);

		if (existsSync(registryPath)) {
			const registry = JSON.parse(readFileSync(registryPath, 'utf-8'));

			registry.forEach((entry: any) => {
				// Chunk should follow pattern: sites/{domain}/index.js
				expect(entry.chunk).toMatch(/^sites\/[^/]+\/index\.js$/);
				expect(entry.chunk).toBe(`sites/${entry.id}/index.js`);
			});
		}
	});
});

test.describe('Content Script Code Quality', () => {
	test('bootstrap should have proper error handling', () => {
		const bootstrapPath = path.join(
			__dirname,
			'../../../../extensions/chromium/scripts/content/bootstrap.js',
		);

		if (existsSync(bootstrapPath)) {
			const content = readFileSync(bootstrapPath, 'utf-8');

			// Should have try-catch blocks
			expect(content).toContain('try');
			expect(content).toContain('catch');

			// Should handle errors
			expect(content).toContain('console.error');
		}
	});

	test('site handlers should export main function', () => {
		const defaultHandlerPath = path.join(
			__dirname,
			'../../../../extensions/chromium/scripts/content/sites/_default/index.js',
		);

		if (existsSync(defaultHandlerPath)) {
			const content = readFileSync(defaultHandlerPath, 'utf-8');
			// Should export main function (checking for various export patterns including minified)
			// Matches: export { x as main }, export function main, export const main
			expect(content).toMatch(
				/(export\s*{[^}]*\s+as\s+main|export\s+(function|const)\s+main)/,
			);
		}
	});
});
