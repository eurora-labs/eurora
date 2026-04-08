import { test as base, chromium, type BrowserContext, type Worker } from '@playwright/test';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export const test = base.extend<{
	context: BrowserContext;
	extensionId: string;
	sw: Worker;
}>({
	// eslint-disable-next-line no-empty-pattern
	context: async ({}, use) => {
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

	extensionId: async ({ sw }, use) => {
		const extensionId = sw.url().split('/')[2];
		await use(extensionId);
	},

	sw: async ({ context }, use) => {
		let [serviceWorker] = context.serviceWorkers();
		if (!serviceWorker) {
			serviceWorker = await context.waitForEvent('serviceworker');
		}
		await use(serviceWorker);
	},
});

export { expect } from '@playwright/test';
