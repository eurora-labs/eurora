import { test as base, chromium, type BrowserContext } from '@playwright/test';
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
}>({
	// Override context to load extension
	context: async ({}, use) => {
		const pathToExtension = path.join(__dirname, '../../../../../extensions/chromium');
		const context = await chromium.launchPersistentContext('', {
			channel: 'chromium',
			args: [
				`--disable-extensions-except=${pathToExtension}`,
				`--load-extension=${pathToExtension}`,
			],
		});
		await use(context);
		await context.close();
	},

	// Get extension ID for testing
	extensionId: async ({ context }, use) => {
		// for manifest v3:
		let [serviceWorker] = context.serviceWorkers();
		if (!serviceWorker) serviceWorker = await context.waitForEvent('serviceworker');

		const extensionId = serviceWorker.url().split('/')[2];
		await use(extensionId);
	},
});

export { expect } from '@playwright/test';
