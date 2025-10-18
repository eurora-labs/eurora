import { test as base, chromium, type BrowserContext, type Worker } from '@playwright/test';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * Extension fixture for loading the browser extension in tests
 */
export const test = base.extend<{
	context: BrowserContext;
	extensionId: string;
	sw: Worker;
}>({
	// Override context to load extension
	context: async ({}, use) => {
		const pathToExtension = path.join(__dirname, '../../../../../extensions/chromium');
		const context = await chromium.launchPersistentContext('', {
			channel: 'chromium',
			headless: true, // Extensions require headed mode
			args: [
				`--disable-extensions-except=${pathToExtension}`,
				`--load-extension=${pathToExtension}`,
				'--no-sandbox', // Required for CI environments
				'--disable-setuid-sandbox',
				'--disable-dev-shm-usage', // Overcome limited resource problems in CI
				'--disable-gpu', // Applicable to CI environments
			],
		});
		try {
			await use(context);
		} finally {
			await context.close();
		}
	},

	// Get extension ID for testing
	// extensionId: async ({ context }, use) => {
	// 	// for manifest v3:
	// 	let [serviceWorker] = context.serviceWorkers();
	// 	if (!serviceWorker) {
	// 		// Wait for service worker with timeout
	// 		serviceWorker = await context.waitForEvent('serviceworker', { timeout: 30000 });
	// 	}

	// 	const extensionId = serviceWorker.url().split('/')[2];
	// 	await use(extensionId);
	// },
	sw: async ({ context }, use) => {
		let [serviceWorker] = context.serviceWorkers();
		if (!serviceWorker) {
			// Wait for service worker with timeout
			serviceWorker = await context.waitForEvent('serviceworker', { timeout: 30000 });
		}
		await use(serviceWorker);
	},
});

export { expect } from '@playwright/test';
