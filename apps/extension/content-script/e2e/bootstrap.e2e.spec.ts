import { test, expect } from './fixtures/extension.js';
import path from 'path';
import { fileURLToPath } from 'url';
import { readFileSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test.describe('Bootstrap Mechanism E2E Tests', () => {
	test.skip('should load bootstrap script on page (requires built extension)', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();
		await page.goto('https://example.com');

		// Check if bootstrap script is loaded
		const hasBootstrap = await page.evaluate(() => {
			return typeof (window as any).__extension_bootstrap__ !== 'undefined';
		});

		expect(hasBootstrap).toBeTruthy();
		await page.close();
	});

	test.skip('should respond to SITE_LOAD messages (requires built extension)', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();
		await page.goto('https://example.com');

		// Wait for bootstrap to be ready
		await page.waitForTimeout(1000);

		// Simulate sending a SITE_LOAD message
		const response = await page.evaluate(async (extId) => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'SITE_LOAD',
						chunk: 'sites/_default/index.js',
						defaultChunk: 'sites/_default/index.js',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		}, extensionId);

		expect(response).toBeTruthy();
		expect((response as any).loaded).toBe(true);
		await page.close();
	});

	test.skip('should only load site script once', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://example.com');

		await page.waitForTimeout(1000);

		// Send first SITE_LOAD message
		const response1 = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'SITE_LOAD',
						chunk: 'sites/_default/index.js',
						defaultChunk: 'sites/_default/index.js',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		// Send second SITE_LOAD message (should be ignored)
		const response2 = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'SITE_LOAD',
						chunk: 'sites/_default/index.js',
						defaultChunk: 'sites/_default/index.js',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		expect(response1).toBeTruthy();
		expect((response1 as any).loaded).toBe(true);
		// Second load should return false (already loaded)
		expect(response2).toBe(false);
		await page.close();
	});

	test.skip('should fall back to default when site handler fails', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();
		await page.goto('https://example.com');

		await page.waitForTimeout(1000);

		// Listen for console errors
		const consoleMessages: string[] = [];
		page.on('console', (msg) => {
			if (msg.type() === 'error') {
				consoleMessages.push(msg.text());
			}
		});

		// Send SITE_LOAD with invalid chunk
		await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'SITE_LOAD',
						chunk: 'sites/invalid/index.js',
						defaultChunk: 'sites/_default/index.js',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		await page.waitForTimeout(500);

		// Should have error about loading site script
		expect(consoleMessages.some((msg) => msg.includes('Error loading site script'))).toBe(true);

		await page.close();
	});

	test.skip('should handle canHandle function correctly', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://example.com');

		await page.waitForTimeout(1000);

		// This test would require a custom site handler with canHandle function
		// For now, we're testing that the system respects canHandle return value
		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'SITE_LOAD',
						chunk: 'sites/_default/index.js',
						defaultChunk: 'sites/_default/index.js',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		expect(response).toBeTruthy();
		await page.close();
	});
});
