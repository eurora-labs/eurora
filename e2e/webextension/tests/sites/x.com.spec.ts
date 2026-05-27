import { test, expect } from '../utils/fixtures.ts';
import {
	getContext,
	invokeTool,
	listTools,
	waitForBootstrap,
	waitForSiteMounted,
} from '../utils/helpers.ts';
import { twitterTools, webToolset, type WireToolDescriptor } from '@eurora/browser/tools';
import type { ToolResult } from '../utils/types.ts';

const SIMPLE_TWEET_URL = 'https://x.com/ptr_to_joel/status/2036935204884152678';
const SIMPLE_TWEET_AUTHOR = '@ptr_to_joel';
const SIMPLE_TWEET_TIMESTAMP = '2026-03-25T00:54:30.000Z';
const SIMPLE_TWEET_ID = '2036935204884152678';

const COMPLEX_TWEET_URL = 'https://x.com/birdabo/status/2041692377967202564';
const COMPLEX_TWEET_AUTHOR = '@birdabo';
const COMPLEX_TWEET_TIMESTAMP = '2026-04-06T14:29:10.000Z';
const COMPLEX_TWEET_ID = '2041692377967202564';

const TWEET_ARTICLE_SELECTOR = 'article[data-testid="tweet"]';

function expectedNameSet(
	...lists: ReadonlyArray<{ descriptor: WireToolDescriptor }>[]
): Set<string> {
	const set = new Set<string>();
	for (const list of lists) {
		for (const tool of list) set.add(tool.descriptor.name);
	}
	return set;
}

function nameSet(descriptors: ReadonlyArray<WireToolDescriptor>): Set<string> {
	return new Set(descriptors.map((d) => d.name));
}

interface TweetCase {
	label: string;
	url: string;
	author: string;
	timestamp: string;
	statusId: string;
}

const TWEET_CASES: TweetCase[] = [
	{
		label: 'simple tweet',
		url: SIMPLE_TWEET_URL,
		author: SIMPLE_TWEET_AUTHOR,
		timestamp: SIMPLE_TWEET_TIMESTAMP,
		statusId: SIMPLE_TWEET_ID,
	},
	{
		label: 'complex tweet',
		url: COMPLEX_TWEET_URL,
		author: COMPLEX_TWEET_AUTHOR,
		timestamp: COMPLEX_TWEET_TIMESTAMP,
		statusId: COMPLEX_TWEET_ID,
	},
];

test.describe('X.com tool surface', { tag: '@x' }, () => {
	for (const { label, url, author, timestamp, statusId } of TWEET_CASES) {
		test.describe(label, () => {
			test.beforeEach(async ({ page }) => {
				await page.goto(url);
				await waitForBootstrap(page);
				await waitForSiteMounted(page, 'x.com');
				await page.waitForSelector(TWEET_ARTICLE_SELECTOR);
			});

			test('advertises the THREAD tool slice for a /status/ page', async ({ sw }) => {
				const advertised = nameSet(await listTools(sw));
				const expected = expectedNameSet(
					webToolset.webTools,
					twitterTools.resolveTwitterTools('/handle/status/1'),
				);
				expect(advertised).toEqual(expected);
				/// Timeline-only tools must not leak onto a thread page —
				/// `twitter_list_timeline_tweets` would happily return the
				/// main tweet + visible replies on a thread, but the LLM
				/// should be using the explicit `twitter_get_tweet_thread`
				/// here so author/replies stay structurally distinguished.
				expect(advertised.has('twitter_list_timeline_tweets')).toBe(false);
				expect(advertised.has('twitter_get_tweet_thread')).toBe(true);
			});

			test('GET_CONTEXT identifies the page as a tweet thread', async ({ sw }) => {
				const ctx = await getContext(sw);
				expect(ctx.blocks).toHaveLength(1);
				const block = ctx.blocks[0];
				expect(block.type).toBe('text');
				expect((block as { type: 'text'; text: string }).text).toMatch(
					/currently reading a tweet thread on X/,
				);
			});

			test('twitter_get_tweet_thread returns the main tweet with structured metadata', async ({
				sw,
			}) => {
				const result = await invokeTool<ToolResult<typeof twitterTools.getTweetThread>>(
					sw,
					'twitter_get_tweet_thread',
				);
				expect(result.main_tweet).not.toBeNull();
				const main = result.main_tweet!;
				expect(main.text).toBeTruthy();
				expect(main.author).toBe(author);
				expect(main.timestamp).toBe(timestamp);
				/// `status_url` is `null` in practice for the main tweet
				/// on a thread page — X.com doesn't wrap its `<time>` in
				/// a permalink anchor when the page IS the tweet's
				/// permalink. When the DOM does happen to include one,
				/// it must point at the same status id we navigated to.
				if (main.status_url !== null) {
					expect(main.status_url).toContain(statusId);
				}
				/// Empty `selector_path` is the safe fallback inside
				/// `safeSelectorPath` when `buildSelectorPath` throws. If
				/// that fallback fires in CI it means the selector-path
				/// builder regressed for an `<article>` it should handle.
				expect(main.selector_path.length).toBeGreaterThan(0);
				expect(Array.isArray(result.replies)).toBe(true);
				for (const reply of result.replies) {
					if (reply.status_url !== null) {
						expect(reply.status_url).toMatch(
							/^https?:\/\/(x|twitter)\.com\/.+\/status\/\d+/,
						);
					}
				}
			});

			test('twitter_get_page_context resolves to a tweet kind', async ({ sw }) => {
				const result = await invokeTool<ToolResult<typeof twitterTools.getPageContext>>(
					sw,
					'twitter_get_page_context',
				);
				expect(result.kind).toBe('tweet');
				expect(result.url).toContain(statusId);
			});
		});
	}

	test.describe('home timeline', () => {
		test.beforeEach(async ({ page }) => {
			await page.goto('https://x.com/home');
			await waitForBootstrap(page);
			await waitForSiteMounted(page, 'x.com');
		});

		test('advertises the TIMELINE tool slice on /home', async ({ sw }) => {
			const advertised = nameSet(await listTools(sw));
			const expected = expectedNameSet(
				webToolset.webTools,
				twitterTools.resolveTwitterTools('/home'),
			);
			expect(advertised).toEqual(expected);
			expect(advertised.has('twitter_list_timeline_tweets')).toBe(true);
			expect(advertised.has('twitter_get_tweet_thread')).toBe(false);
		});

		test('twitter_list_timeline_tweets returns rendered tweets when logged in', async ({
			page,
			sw,
		}) => {
			/// The home timeline only renders tweets for authenticated
			/// sessions. The fresh persistent context Playwright spins up
			/// has no cookies, so X redirects to the login wall. Skip
			/// instead of failing — local runs with a hand-stashed cookie
			/// jar still exercise the path.
			const rendered = await page.locator(TWEET_ARTICLE_SELECTOR).count();
			test.skip(rendered === 0, 'no tweets rendered — likely a logged-out session');

			const result = await invokeTool<ToolResult<typeof twitterTools.listTimelineTweets>>(
				sw,
				'twitter_list_timeline_tweets',
				{ limit: 5 },
			);
			expect(result.tweets.length).toBeGreaterThan(0);
			expect(result.tweets.length).toBeLessThanOrEqual(5);
			expect(result.total).toBeGreaterThanOrEqual(result.tweets.length);
			for (const tweet of result.tweets) {
				expect(tweet.text.length).toBeGreaterThan(0);
				expect(typeof tweet.selector_path).toBe('string');
			}
		});
	});
});
