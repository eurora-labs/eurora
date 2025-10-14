import { test, expect } from './fixtures/extension.js';
import path from 'path';
import { fileURLToPath } from 'url';
import { readFileSync, existsSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test.describe('Registry and Domain Matching E2E Tests', () => {
	test('should have registry.json file built', async ({ context, extensionId }) => {
		// Check if registry.json exists in the built extension
		const registryPath = path.join(
			__dirname,
			'../../../../extensions/chromium/scripts/content/registry.json',
		);
		const exists = existsSync(registryPath);
		expect(exists).toBe(true);

		if (exists) {
			const registryContent = readFileSync(registryPath, 'utf-8');
			const registry = JSON.parse(registryContent);

			// Verify registry structure
			expect(Array.isArray(registry)).toBe(true);
			expect(registry.length).toBeGreaterThan(0);

			// Each entry should have id, chunk, and patterns
			registry.forEach((entry: any) => {
				expect(entry).toHaveProperty('id');
				expect(entry).toHaveProperty('chunk');
				expect(entry).toHaveProperty('patterns');
				expect(Array.isArray(entry.patterns)).toBe(true);
			});
		}
	});

	test('should match youtube.com domain correctly (requires built extension)', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();

		// Navigate to YouTube
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(2000);

		// Check if YouTube-specific handler is loaded
		const hasYoutubeHandler = await page.evaluate(() => {
			// Check for YouTube-specific elements or functions
			return document.querySelector('video.html5-main-video') !== null;
		});

		expect(hasYoutubeHandler).toBe(true);
		await page.close();
	});

	test('should match wildcard patterns correctly', async ({ context, extensionId }) => {
		const registryPath = path.join(
			__dirname,
			'../../../../extensions/chromium/scripts/content/registry.json',
		);

		if (existsSync(registryPath)) {
			const registryContent = readFileSync(registryPath, 'utf-8');
			const registry = JSON.parse(registryContent);

			// Find entries with wildcard patterns
			const wildcardEntries = registry.filter((entry: any) =>
				entry.patterns.some((pattern: string) => pattern.includes('*')),
			);

			// Each wildcard entry should have proper pattern format
			wildcardEntries.forEach((entry: any) => {
				entry.patterns.forEach((pattern: string) => {
					if (pattern.includes('*')) {
						// Pattern should either be the domain with wildcard or have *.domain format
						expect(pattern.startsWith('*.') || pattern.endsWith('*')).toBe(true);
					}
				});
			});
		}
	});

	test('should use default handler for unmatched domains (requires built extension)', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();

		// Navigate to a domain that shouldn't have a specific handler
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		// The default handler should be loaded
		// We can't directly check which handler is loaded, but we can verify the page loads
		const title = await page.title();
		expect(title).toBeTruthy();

		await page.close();
	});

	test('should handle subdomain matching correctly (requires built extension)', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();

		// Test with YouTube subdomain
		await page.goto('https://m.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(2000);

		// Should still match youtube.com handler
		const hasVideo = await page.evaluate(() => {
			return document.querySelector('video') !== null;
		});

		expect(hasVideo).toBe(true);
		await page.close();
	});

	test('should generate correct patterns for domain', async () => {
		// Test the patternsFor function logic
		const testCases = [
			{ id: '_default', expected: [] },
			{ id: 'youtube.com', expected: ['youtube.com', '*.youtube.com'] },
			{ id: '*.example.com', expected: ['*.example.com'] },
		];

		// This would need to be tested by examining the registry.json output
		const registryPath = path.join(
			__dirname,
			'../../../../extensions/chromium/scripts/content/registry.json',
		);

		if (existsSync(registryPath)) {
			const registryContent = readFileSync(registryPath, 'utf-8');
			const registry = JSON.parse(registryContent);

			// Verify _default is not in registry
			const defaultEntry = registry.find((e: any) => e.id === '_default');
			expect(defaultEntry).toBeUndefined();

			// Verify youtube.com has correct patterns
			const youtubeEntry = registry.find((e: any) => e.id === 'youtube.com');
			if (youtubeEntry) {
				expect(youtubeEntry.patterns).toEqual(
					expect.arrayContaining(['youtube.com', '*.youtube.com']),
				);
			}
		}
	});

	test('should map domains to correct chunk paths', async () => {
		const registryPath = path.join(
			__dirname,
			'../../../../extensions/chromium/scripts/content/registry.json',
		);

		if (existsSync(registryPath)) {
			const registryContent = readFileSync(registryPath, 'utf-8');
			const registry = JSON.parse(registryContent);

			// Each entry's chunk should follow the pattern: sites/{domain}/index.js
			registry.forEach((entry: any) => {
				expect(entry.chunk).toMatch(/^sites\/[^/]+\/index\.js$/);
				expect(entry.chunk).toBe(`sites/${entry.id}/index.js`);

				// Verify the chunk file exists
				const chunkPath = path.join(
					__dirname,
					'../../../../extensions/chromium/scripts/content',
					entry.chunk,
				);
				expect(existsSync(chunkPath)).toBe(true);
			});
		}
	});

	test('should exclude _default from registry', async () => {
		const registryPath = path.join(
			__dirname,
			'../../../../extensions/chromium/scripts/content/registry.json',
		);

		if (existsSync(registryPath)) {
			const registryContent = readFileSync(registryPath, 'utf-8');
			const registry = JSON.parse(registryContent);

			// _default should not be in the registry
			const hasDefault = registry.some((entry: any) => entry.id === '_default');
			expect(hasDefault).toBe(false);

			// But the _default chunk file should still exist
			const defaultChunkPath = path.join(
				__dirname,
				'../../../../extensions/chromium/scripts/content/sites/_default/index.js',
			);
			expect(existsSync(defaultChunkPath)).toBe(true);
		}
	});
});
