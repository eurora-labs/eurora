import { test, expect } from '../utils/fixtures.ts';
import {
	getContext,
	invokeTool,
	listTools,
	waitForBootstrap,
	waitForSiteMounted,
} from '../utils/helpers.ts';
import { webToolset, type WireToolDescriptor } from '@eurora/browser/tools';
import type { ToolResult } from '../utils/types.ts';

const PAGE_URL = 'https://example.com/';

function nameSet(descriptors: ReadonlyArray<WireToolDescriptor>): Set<string> {
	return new Set(descriptors.map((d) => d.name));
}

test.describe('Default site tool surface', { tag: '@default' }, () => {
	test.beforeEach(async ({ page }) => {
		await page.goto(PAGE_URL);
		await waitForBootstrap(page);
		await waitForSiteMounted(page, 'default');
	});

	test('advertises exactly the generic webTools set', async ({ sw }) => {
		const advertised = nameSet(await listTools(sw));
		const expected = new Set(webToolset.webTools.map((t) => t.descriptor.name));
		expect(advertised).toEqual(expected);
	});

	test('every advertised descriptor is sourced from the browser bridge', async ({ sw }) => {
		/// Pins the bridge-side contract: every default-page tool must
		/// route through the browser native-messaging host with
		/// `app_kind: 'browser'`. A regression here would mean the
		/// desktop side starts trying to handle a tool locally and the
		/// extension never sees the call.
		const tools = await listTools(sw);
		for (const tool of tools) {
			expect(tool.source).toEqual({ kind: 'bridge', app_kind: 'browser' });
			expect(tool.timeout_ms).toBeGreaterThan(0);
		}
	});

	test('GET_CONTEXT describes the page with title and url', async ({ sw }) => {
		const ctx = await getContext(sw);
		expect(ctx.blocks).toHaveLength(1);
		const block = ctx.blocks[0];
		expect(block.type).toBe('text');
		const text = (block as { type: 'text'; text: string }).text;
		expect(text).toMatch(
			/^The user is on the web page "Example Domain" at https:\/\/example\.com/,
		);
		expect(text).not.toContain('highlighted');
	});

	test('GET_CONTEXT appends a highlight clause once text is selected', async ({ page, sw }) => {
		await page.evaluate(() => {
			const p = document.querySelector('p');
			if (!p) throw new Error('expected a <p> in example.com');
			const range = document.createRange();
			range.selectNodeContents(p);
			const sel = window.getSelection();
			sel?.removeAllRanges();
			sel?.addRange(range);
		});
		const ctx = await getContext(sw);
		const text = (ctx.blocks[0] as { type: 'text'; text: string }).text;
		expect(text).toContain('They have the following text highlighted:');
	});

	test('web_get_page_metadata returns the document title and host', async ({ sw }) => {
		const result = await invokeTool<ToolResult<typeof webToolset.getPageMetadata>>(
			sw,
			'web_get_page_metadata',
		);
		expect(result.title).toBe('Example Domain');
		expect(result.host).toBe('example.com');
		expect(result.url).toBe(PAGE_URL);
		expect(result.viewport.inner_width).toBeGreaterThan(0);
	});

	test('web_get_readability_article returns the body text', async ({ sw }) => {
		const result = await invokeTool<ToolResult<typeof webToolset.getReadabilityArticle>>(
			sw,
			'web_get_readability_article',
		);
		expect(result.text_content.toLowerCase()).toContain('this domain is for use in');
		expect(result.length).toBeGreaterThan(0);
	});

	test('web_list_links surfaces the IANA reference link', async ({ sw }) => {
		const result = await invokeTool<ToolResult<typeof webToolset.listLinks>>(
			sw,
			'web_list_links',
		);
		expect(result.total).toBeGreaterThan(0);
		const ianaLink = result.links.find((l) => l.url.includes('iana.org'));
		expect(ianaLink, 'expected the IANA link from example.com').toBeDefined();
		expect(ianaLink!.label?.toLowerCase()).toMatch(/learn more|more information/);
	});

	test('web_query_selector returns a paragraph match with a selector path', async ({ sw }) => {
		const result = await invokeTool<ToolResult<typeof webToolset.querySelector>>(
			sw,
			'web_query_selector',
			{ selector: 'p', limit: 5, include: ['text'] },
		);
		expect(result.total_match_count).toBeGreaterThan(0);
		expect(result.matches.length).toBeGreaterThan(0);
		expect(result.matches[0].selector_path.length).toBeGreaterThan(0);
		expect(result.matches[0].text).not.toBeNull();
		expect(result.matches[0].text!.length).toBeGreaterThan(0);
	});

	test('web_get_selected_text returns whatever the user has highlighted', async ({
		page,
		sw,
	}) => {
		const selectedText = await page.evaluate(() => {
			const p = document.querySelector('p');
			if (!p) throw new Error('expected a <p> in example.com');
			const range = document.createRange();
			range.selectNodeContents(p);
			const sel = window.getSelection();
			sel?.removeAllRanges();
			sel?.addRange(range);
			return range.toString();
		});

		const result = await invokeTool<ToolResult<typeof webToolset.getSelectedText>>(
			sw,
			'web_get_selected_text',
		);
		expect(result.text).toBe(selectedText);
		expect(result.anchor_xpath).not.toBeNull();
		expect(result.focus_xpath).not.toBeNull();
	});

	test('web_get_selected_text returns empty fields when nothing is highlighted', async ({
		page,
		sw,
	}) => {
		await page.evaluate(() => window.getSelection()?.removeAllRanges());
		const result = await invokeTool<ToolResult<typeof webToolset.getSelectedText>>(
			sw,
			'web_get_selected_text',
		);
		expect(result.text).toBe('');
	});
});
