import { test, expect } from '../utils/fixtures.ts';
import { waitForBootstrap, waitForSiteMounted, sendToActiveTab } from '../utils/helpers.ts';

test.describe('Default Site Watcher Tests', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('https://example.com');
		await waitForBootstrap(page);
		await waitForSiteMounted(page, 'default');
	});

	test('should extract article from page', async ({ sw }) => {
		const response = await sendToActiveTab(sw, { type: 'GENERATE_ASSETS' });

		if (!response) {
			throw new Error('Expected a response from GENERATE_ASSETS');
		}

		expect(response.kind).toEqual('NativeArticleAsset');
		expect(response.data.title).toBeDefined();
		expect(response.data.text_content).toContain('This domain is for use in');
	});

	test('should extract selected text from page', async ({ page, sw }) => {
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

		const response = await sendToActiveTab(sw, { type: 'GENERATE_SNAPSHOT' });

		if (!response) {
			throw new Error('Expected a response from GENERATE_SNAPSHOT');
		}

		expect(response.kind).toEqual('NativeArticleSnapshot');
		expect(response.data.highlighted_text).toEqual(selectedText);
	});
});
