/* eslint-disable @typescript-eslint/ban-ts-comment, no-empty-pattern */
import { test as base, chromium, type BrowserContext, type Worker } from '@playwright/test';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * Extension fixture for loading the browser extension in tests
 *
 * This fixture provides:
 * - context: A persistent browser context with the extension loaded
 * - extensionId: The ID of the loaded extension
 * - sw: The service worker (background script) of the extension
 */
export const test = base.extend<{
	context: BrowserContext;
	extensionId: string;
	sw: Worker;
}>({
	// Override context to load the complete extension (content scripts, background, popup)
	// @ts-ignore
	context: async ({}, use) => {
		// Path to the built extension directory
		const pathToExtension = path.join(__dirname, '../../../../apps/browser/dist/chrome');

		const context = await chromium.launchPersistentContext('', {
			channel: 'chromium',
			args: [
				`--disable-extensions-except=${pathToExtension}`,
				`--load-extension=${pathToExtension}`,
			],
		});

		try {
			await use(context);
		} finally {
			await context.close();
		}
	},

	// Get extension ID from the service worker URL
	extensionId: async ({ context }, use) => {
		// For manifest v3: Get the service worker
		let [serviceWorker] = context.serviceWorkers();
		if (!serviceWorker) {
			serviceWorker = await context.waitForEvent('serviceworker');
		}

		const extensionId = serviceWorker.url().split('/')[2];
		await use(extensionId);
	},

	// Provide direct access to the service worker for testing background script functionality
	sw: async ({ context }, use) => {
		let [serviceWorker] = context.serviceWorkers();
		if (!serviceWorker) {
			serviceWorker = await context.waitForEvent('serviceworker');
		}
		await use(serviceWorker);
	},
});

export { expect } from '@playwright/test';
