import { test, expect } from '../utils/fixtures.ts';
import { waitForBootstrap, waitForSiteMounted, sendToActiveTab } from '../utils/helpers.ts';
import type { NativeTwitterAsset, TweetPageData } from '../utils/types.ts';

const SIMPLE_TWEET_URL = 'https://x.com/ptr_to_joel/status/2036935204884152678';
const COMPLEX_TWEET_URL = 'https://x.com/birdabo/status/2041692377967202564';

test.describe('X.com Watcher Tests', { tag: '@x' }, () => {
	test.describe('simple tweet', () => {
		test.beforeEach(async ({ page }) => {
			await page.goto(SIMPLE_TWEET_URL);
			await waitForBootstrap(page);
			await waitForSiteMounted(page, 'x.com');
			await page.waitForSelector('article[data-testid="tweet"]');
		});

		test('should extract tweet asset with correct structure', async ({ sw }) => {
			const response = await sendToActiveTab(sw, { type: 'GENERATE_ASSETS' });

			if (!response) {
				throw new Error('Expected a response from GENERATE_ASSETS');
			}

			expect(response.kind).toEqual('NativeTwitterAsset');

			const asset = response.data as NativeTwitterAsset;
			expect(asset.url).toContain('ptr_to_joel/status/2036935204884152678');
			expect(asset.title).toBeTruthy();
			expect(asset.timestamp).toBeTruthy();
			expect(asset.result.page).toEqual('tweet');
		});

		test('should extract main tweet with text content', async ({ sw }) => {
			const response = await sendToActiveTab(sw, { type: 'GENERATE_ASSETS' });

			if (!response) {
				throw new Error('Expected a response from GENERATE_ASSETS');
			}

			const { tweet, replies } = (response.data as NativeTwitterAsset).result
				.data as TweetPageData;

			expect(tweet).not.toBeNull();
			expect(tweet!.text).toBeTruthy();
			expect(tweet!.author).toEqual('@ptr_to_joel');
			expect(tweet!.timestamp).toEqual('2026-03-25T00:54:30.000Z');
			expect(Array.isArray(replies)).toBe(true);
		});
	});

	test.describe('complex tweet', () => {
		test.beforeEach(async ({ page }) => {
			await page.goto(COMPLEX_TWEET_URL);
			await waitForBootstrap(page);
			await waitForSiteMounted(page, 'x.com');
			await page.waitForSelector('article[data-testid="tweet"]');
		});

		test('should extract tweet asset with correct structure', async ({ sw }) => {
			const response = await sendToActiveTab(sw, { type: 'GENERATE_ASSETS' });

			if (!response) {
				throw new Error('Expected a response from GENERATE_ASSETS');
			}

			expect(response.kind).toEqual('NativeTwitterAsset');

			const asset = response.data as NativeTwitterAsset;
			expect(asset.url).toContain('birdabo/status/2041692377967202564');
			expect(asset.title).toBeTruthy();
			expect(asset.timestamp).toBeTruthy();
			expect(asset.result.page).toEqual('tweet');
		});

		test('should extract main tweet with text content', async ({ sw }) => {
			const response = await sendToActiveTab(sw, { type: 'GENERATE_ASSETS' });

			if (!response) {
				throw new Error('Expected a response from GENERATE_ASSETS');
			}

			const { tweet, replies } = (response.data as NativeTwitterAsset).result
				.data as TweetPageData;

			expect(tweet).not.toBeNull();
			expect(tweet!.text).toBeTruthy();
			expect(tweet!.author).toEqual('@birdabo');
			expect(tweet!.timestamp).toEqual('2026-04-06T14:29:10.000Z');
			expect(Array.isArray(replies)).toBe(true);
		});
	});
});
