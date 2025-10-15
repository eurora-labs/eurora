import { WatcherResponse } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import { test, expect } from '../util/fixtures.ts';
import { waitForBootstrap, waitForSiteMounted } from '../util/helpers.ts';

test.describe('Youtube Watcher Tests', () => {
	test('Should extract English subtitles from a video', async ({ page, sw }) => {
		await page.goto('https://www.youtube.com/watch?v=CXKoCMVqM9s&t=1289s');
		await waitForBootstrap(page);
		await waitForSiteMounted(page, 'youtube.com');

		const response: WatcherResponse = await sw.evaluate(async () => {
			const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
			return await new Promise((resolve) => {
				chrome.tabs.sendMessage(tab.id!, { type: 'GENERATE_ASSETS' }, (response) =>
					resolve(response),
				);
			});
		});

		expect(response).toBeDefined();
		expect(response.kind).toEqual('NativeYoutubeAsset');
		expect(response.data).toBeDefined();
		expect(Array.isArray(JSON.parse(response.data.transcript))).toEqual(true);
		expect(response.data.current_time).toEqual(1289);
	});
});
