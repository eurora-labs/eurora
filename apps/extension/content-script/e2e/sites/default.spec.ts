import { WatcherResponse } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import { test, expect } from '../util/fixtures.ts';
import { waitForBootstrap, waitForSiteMounted } from '../util/helpers.ts';

test.describe('Article Watcher Tests', () => {
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
	test('should extract selected text from page', async ({ page, sw }) => {
		await page.goto('https://example.com');
		await waitForBootstrap(page);
		await waitForSiteMounted(page, 'default');

		// Initialize selection and range

		const selectedText = await page.evaluate(() => {
			const p = document.querySelector('p');

			if (!p) return undefined;

			const selection = window.getSelection();
			const range = document.createRange();

			range.selectNodeContents(p);
			selection?.removeAllRanges();
			selection?.addRange(range);
			return range.toString();
		});

		expect(selectedText).toBeDefined();

		const response: WatcherResponse = await sw.evaluate(async () => {
			const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
			return await new Promise((resolve) => {
				chrome.tabs.sendMessage(tab.id!, { type: 'GENERATE_SNAPSHOT' }, (response) =>
					resolve(response),
				);
			});
		});

		expect(response).toBeDefined();
		expect(response.kind).toEqual('NativeArticleSnapshot');
		expect(response.data).toBeDefined();
		expect(response.data.highlighted_text).toEqual(selectedText);
	});
});
