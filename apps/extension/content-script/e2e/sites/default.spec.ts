import { WatcherResponse } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import { test, expect } from '../util/fixtures.ts';
import { waitForBootstrap, waitForSiteMounted } from '../util/helpers.ts';

test.describe('Content Script Basic Tests', () => {
	test('should extract article from page', async ({ page, sw }) => {
		await page.goto('https://example.com');
		await waitForBootstrap(page);
		await waitForSiteMounted(page, 'default');

		const response: WatcherResponse = await sw.evaluate(async () => {
			const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
			return await new Promise((resolve) => {
				chrome.tabs.sendMessage(tab.id!, { type: 'GENERATE_ASSETS' }, (response) =>
					resolve(response),
				);
			});
		});

		expect(response).toBeDefined();
		expect(response.kind).toEqual('NativeArticleAsset');
		expect(response.data.title).toBeDefined();
		expect(response.data.text_content).toBeDefined();
		expect(response.data.text_content).toContain('This domain is for use in');
	});
});
