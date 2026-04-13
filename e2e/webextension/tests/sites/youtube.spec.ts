import { test, expect } from '../utils/fixtures.ts';
import { waitForBootstrap, waitForSiteMounted, sendToActiveTab } from '../utils/helpers.ts';
import type { TranscriptSnippet } from '../utils/types.ts';

const VIDEO_URL = 'https://www.youtube.com/watch?v=CXKoCMVqM9s&t=1289s';

test.describe('Youtube Watcher Tests', { tag: '@youtube' }, () => {
	test.describe('watch page', () => {
		test.beforeEach(async ({ page }) => {
			await page.goto(VIDEO_URL);
			await waitForBootstrap(page);
			await waitForSiteMounted(page, 'youtube.com');
		});

		test('should extract video asset with all fields', async ({ sw }) => {
			const response = await sendToActiveTab(sw, { type: 'GENERATE_ASSETS' });

			if (!response) {
				throw new Error('Expected a response from GENERATE_ASSETS');
			}

			expect(response.kind).toEqual('NativeYoutubeAsset');

			expect(response.data.url).toContain('youtube.com/watch');
			expect(response.data.title).toBeTruthy();
			expect(response.data.current_time).toEqual(1289);

			const snippets: TranscriptSnippet[] = JSON.parse(response.data.transcript);
			expect(snippets.length).toBeGreaterThan(0);
			expect(snippets[0]).toEqual(
				expect.objectContaining({
					text: expect.any(String),
					start: expect.any(Number),
					duration: expect.any(Number),
				}),
			);
		});

		test('should extract video snapshot with all fields', async ({ sw }) => {
			const response = await sendToActiveTab(sw, { type: 'GENERATE_SNAPSHOT' });

			if (!response) {
				throw new Error('Expected a response from GENERATE_SNAPSHOT');
			}

			expect(response.kind).toEqual('NativeYoutubeSnapshot');

			expect(response.data.current_time).toBeGreaterThanOrEqual(0);
			expect(response.data.video_frame_base64.length).toBeGreaterThan(0);
			expect(response.data.video_frame_width).toBeGreaterThan(0);
			expect(response.data.video_frame_height).toBeGreaterThan(0);
		});
	});

	test.describe('non-watch page', () => {
		test('should fall back to article extraction', async ({ page, sw }) => {
			await page.goto('https://www.youtube.com');
			await waitForBootstrap(page);
			await waitForSiteMounted(page, 'youtube.com');

			const response = await sendToActiveTab(sw, { type: 'GENERATE_ASSETS' });

			if (!response) {
				throw new Error('Expected a response from GENERATE_ASSETS');
			}

			expect(response.kind).toEqual('NativeArticleAsset');
			expect(response.data.title).toBeTruthy();
			expect(response.data.text_content).toBeDefined();
		});
	});
});
