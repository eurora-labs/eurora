import { test, expect } from '../utils/fixtures.ts';
import { waitForBootstrap, waitForSiteMounted } from '../utils/helpers.ts';
import { WatcherResponse } from '../utils/types.ts';

test.describe('Youtube Watcher Tests', { tag: '@youtube' }, () => {
	test('should extract English subtitles from a video', async ({ page, sw }) => {
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
		if (response === undefined) {
			throw new Error('Response is undefined');
		}

		expect(response.kind).toEqual('NativeYoutubeAsset');
		expect(response.data).toBeDefined();
		expect(Array.isArray(JSON.parse(response.data.transcript))).toEqual(true);
		expect(response.data.current_time).toEqual(1289);
	});

	test('should extract video frame from a video', async ({ page, sw }) => {
		await page.goto('https://www.youtube.com/watch?v=CXKoCMVqM9s&t=1289s');
		await waitForBootstrap(page);
		await waitForSiteMounted(page, 'youtube.com');

		const response: WatcherResponse = await sw.evaluate(async () => {
			const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
			return await new Promise((resolve) => {
				chrome.tabs.sendMessage(tab.id!, { type: 'GENERATE_SNAPSHOT' }, (response) =>
					resolve(response),
				);
			});
		});

		expect(response).toBeDefined();
		if (response === undefined) {
			throw new Error('Response is undefined');
		}

		expect(response.kind).toEqual('NativeYoutubeSnapshot');
		expect(response.data).toBeDefined();
		expect(response.data.video_frame_base64).toBeDefined();
		expect(response.data.video_frame_base64.length).toBeGreaterThan(0);
	});
});
