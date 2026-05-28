import { test, expect } from '../utils/fixtures.ts';
import {
	getContext,
	invokeTool,
	listTools,
	waitForBootstrap,
	waitForSiteMounted,
} from '../utils/helpers.ts';
import { webToolset, youtubeTools, type WireToolDescriptor } from '@eurora/browser/tools';
import type { ToolResult, TranscriptSnippet } from '../utils/types.ts';

const WATCH_URL = 'https://www.youtube.com/watch?v=CXKoCMVqM9s&t=1289s';
const WATCH_DEEP_LINK_SECONDS = 1289;
/// `formatHms(1289)` — the watcher's context summary stamps the deep-link
/// timestamp into the sentence the LLM sees. Pinning this prevents a
/// regression in the (separately unit-tested) formatter from going
/// unnoticed downstream.
const WATCH_TIMESTAMP_HMS = '21:29';

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

test.describe('YouTube tool surface', { tag: '@youtube' }, () => {
	test.describe('watch page', () => {
		test.beforeEach(async ({ page }) => {
			await page.goto(WATCH_URL);
			await waitForBootstrap(page);
			await waitForSiteMounted(page, 'youtube.com');
		});

		test('advertises the WATCH tool slice on top of the generic web tools', async ({ sw }) => {
			const advertised = nameSet(await listTools(sw));
			const expected = expectedNameSet(
				webToolset.webTools,
				youtubeTools.resolveYoutubeTools('/watch'),
			);
			expect(advertised).toEqual(expected);
		});

		test('GET_CONTEXT stamps the watch-page summary with the deep-link timestamp', async ({
			sw,
		}) => {
			const ctx = await getContext(sw);
			expect(ctx.blocks).toHaveLength(1);
			const block = ctx.blocks[0];
			expect(block.type).toBe('text');
			const text = (block as { type: 'text'; text: string }).text;
			expect(text).toMatch(/currently watching a YouTube video/);
			expect(text).toContain(`at timestamp ${WATCH_TIMESTAMP_HMS}`);
		});

		test('youtube_get_current_timestamp reports the deep-link position', async ({ sw }) => {
			const result = await invokeTool<ToolResult<typeof youtubeTools.getCurrentTimestamp>>(
				sw,
				'youtube_get_current_timestamp',
			);
			expect(result.video_id).toBe('CXKoCMVqM9s');
			/// `?t=1289s` seeks to 1289s; the player can drift a few hundred
			/// ms past it before the tool reads `currentTime`. Allow a
			/// generous upper bound so brief CI hangs don't flake.
			expect(result.current_time).toBeGreaterThanOrEqual(WATCH_DEEP_LINK_SECONDS);
			expect(result.current_time).toBeLessThan(WATCH_DEEP_LINK_SECONDS + 30);
			expect(result.duration).not.toBeNull();
			expect(result.duration ?? 0).toBeGreaterThan(0);
			expect(typeof result.playing).toBe('boolean');
		});

		test('youtube_get_video_metadata surfaces title, channel, and counts', async ({
			page,
			sw,
		}) => {
			/// The channel-anchor render is what populates `channel_name`
			/// / `channel_url` / `channel_handle`. Wait for it scoped to
			/// this test only — other watch-page tools work fine without
			/// the deeper hydration and shouldn't pay the cost.
			await page.waitForSelector('ytd-watch-metadata #owner ytd-channel-name a', {
				state: 'attached',
			});
			const result = await invokeTool<ToolResult<typeof youtubeTools.getVideoMetadata>>(
				sw,
				'youtube_get_video_metadata',
			);
			expect(result.video_id).toBe('CXKoCMVqM9s');
			expect(result.title).toBeTruthy();
			expect(result.channel_handle).toMatch(/^@/);
			expect(result.channel_url).toMatch(/^https?:\/\//);
			/// `published_at` and `view_count` come from the hidden
			/// microformat block; both are advertised as nullable in the
			/// tool's output schema. Don't pin a value — just check the
			/// shape when present.
			if (result.published_at !== null) {
				expect(result.published_at).toMatch(/^\d{4}-\d{2}-\d{2}/);
			}
			if (result.view_count !== null) {
				expect(result.view_count).toBeGreaterThanOrEqual(0);
			}
		});

		test('youtube_get_transcript returns plain-text captions', async ({ sw }) => {
			const result = await invokeTool<ToolResult<typeof youtubeTools.getTranscript>>(
				sw,
				'youtube_get_transcript',
			);
			expect(result.video_id).toBe('CXKoCMVqM9s');
			expect(result.language).toBe('en');
			expect(typeof result.is_generated).toBe('boolean');
			expect(result.text.length).toBeGreaterThan(0);
		});

		test('youtube_get_timed_transcript returns per-entry timing data', async ({ sw }) => {
			const result = await invokeTool<ToolResult<typeof youtubeTools.getTimedTranscript>>(
				sw,
				'youtube_get_timed_transcript',
			);
			expect(result.entries.length).toBeGreaterThan(0);
			const sample: TranscriptSnippet = result.entries[0];
			expect(typeof sample.text).toBe('string');
			expect(typeof sample.start).toBe('number');
			expect(typeof sample.duration).toBe('number');
			expect(sample.start).toBeGreaterThanOrEqual(0);
			expect(sample.duration).toBeGreaterThanOrEqual(0);
		});

		test('youtube_get_current_frame captures a base64-encoded PNG', async ({ sw }) => {
			const result = await invokeTool<ToolResult<typeof youtubeTools.getCurrentFrame>>(
				sw,
				'youtube_get_current_frame',
			);
			expect(result.video_id).toBe('CXKoCMVqM9s');
			expect(result.current_time).toBeGreaterThanOrEqual(0);
			expect(result.width).toBeGreaterThan(0);
			expect(result.height).toBeGreaterThan(0);
			expect(result.image_base64.length).toBeGreaterThan(0);
			/// PNGs always start with `iVBORw0KGgo` when base64-encoded
			/// from the standard `89 50 4E 47 0D 0A 1A 0A` magic header.
			expect(result.image_base64.startsWith('iVBORw')).toBe(true);
		});
	});

	test.describe('home page', () => {
		test.beforeEach(async ({ page }) => {
			await page.goto('https://www.youtube.com');
			await waitForBootstrap(page);
			await waitForSiteMounted(page, 'youtube.com');
		});

		test('advertises only the BASE youtube slice plus generic web tools', async ({ sw }) => {
			const advertised = nameSet(await listTools(sw));
			const expected = expectedNameSet(
				webToolset.webTools,
				youtubeTools.resolveYoutubeTools('/'),
			);
			expect(advertised).toEqual(expected);
			/// Player-bound tools must NOT leak onto a page without a player.
			/// If they did, the LLM could call them and the extension
			/// would throw a confusing "player not ready" error instead of
			/// reporting that the tool isn't available.
			expect(advertised.has('youtube_get_current_timestamp')).toBe(false);
			expect(advertised.has('youtube_get_current_frame')).toBe(false);
			expect(advertised.has('youtube_get_transcript')).toBe(false);
		});

		test('web_get_readability_article returns a generic article shape', async ({ sw }) => {
			const result = await invokeTool<ToolResult<typeof webToolset.getReadabilityArticle>>(
				sw,
				'web_get_readability_article',
			);
			expect(typeof result.text_content).toBe('string');
			expect(typeof result.length).toBe('number');
		});

		test('youtube_get_page_context identifies the home page', async ({ sw }) => {
			const result = await invokeTool<ToolResult<typeof youtubeTools.getPageContext>>(
				sw,
				'youtube_get_page_context',
			);
			expect(result.kind).toBe('home');
			expect(result.video_id).toBeNull();
			expect(result.url).toMatch(/^https:\/\/www\.youtube\.com/);
		});
	});
});
