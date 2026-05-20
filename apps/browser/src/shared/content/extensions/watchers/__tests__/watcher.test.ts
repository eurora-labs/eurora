import { Watcher, type BrowserObj, type WatcherResponse } from '../watcher';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import type browser from 'webextension-polyfill';

class TestWatcher extends Watcher<Record<string, never>> {
	public newCalls = 0;

	public async handleNew(): Promise<WatcherResponse> {
		this.newCalls += 1;
		return { kind: 'Ok', data: null };
	}
}

const SENDER = {} as browser.Runtime.MessageSender;

async function call(watcher: Watcher<Record<string, never>>, obj: BrowserObj): Promise<unknown> {
	const result = watcher.listen(obj, SENDER);
	if (result === false) {
		throw new Error(`listen returned false for ${obj.type}`);
	}
	return await result;
}

describe('base Watcher dispatch', () => {
	let watcher: TestWatcher;

	beforeEach(() => {
		document.body.innerHTML = '';
		watcher = new TestWatcher({});
	});

	it('routes NEW to handleNew', async () => {
		await call(watcher, { type: 'NEW' });
		expect(watcher.newCalls).toBe(1);
	});

	it('returns false for an unknown message type so the bus can ignore it', () => {
		const result = watcher.listen({ type: 'NOT_A_THING' }, SENDER);
		expect(result).toBe(false);
	});

	it('dispatches GET_PAGE_METADATA to the web handler', async () => {
		document.title = 'Hello';
		const reply = (await call(watcher, { type: 'GET_PAGE_METADATA' })) as {
			kind: string;
			data: { title: string };
		};
		expect(reply.kind).toBe('PageMetadata');
		expect(reply.data.title).toBe('Hello');
	});

	it('dispatches GET_SELECTED_TEXT to the web handler', async () => {
		const reply = (await call(watcher, { type: 'GET_SELECTED_TEXT' })) as { kind: string };
		expect(reply.kind).toBe('SelectedText');
	});

	it('dispatches QUERY_SELECTOR with caller args', async () => {
		document.body.innerHTML = '<p>x</p>';
		const reply = (await call(watcher, {
			type: 'QUERY_SELECTOR',
			selector: 'p',
			include: ['text'],
		})) as { kind: string; data: { matches: { text: string }[] } };
		expect(reply.kind).toBe('QuerySelectorResult');
		expect(reply.data.matches[0].text).toBe('x');
	});

	it('dispatches LIST_LINKS', async () => {
		document.body.innerHTML = '<a href="https://example.com">x</a>';
		const reply = (await call(watcher, { type: 'LIST_LINKS' })) as { kind: string };
		expect(reply.kind).toBe('LinksList');
	});

	it('dispatches LIST_FORM_INPUTS', async () => {
		document.body.innerHTML = '<input type="text">';
		const reply = (await call(watcher, { type: 'LIST_FORM_INPUTS' })) as { kind: string };
		expect(reply.kind).toBe('FormInputsList');
	});

	it('dispatches GET_ACCESSIBILITY_TREE', async () => {
		document.body.innerHTML = '<main></main>';
		const reply = (await call(watcher, { type: 'GET_ACCESSIBILITY_TREE' })) as { kind: string };
		expect(reply.kind).toBe('AccessibilityTree');
	});

	it('dispatches GET_READABILITY_ARTICLE', async () => {
		document.body.innerHTML = '<article><h1>Title</h1><p>Body.</p></article>';
		const reply = (await call(watcher, { type: 'GET_READABILITY_ARTICLE' })) as {
			kind: string;
		};
		expect(reply.kind).toBe('ReadabilityArticle');
	});

	it('dispatches INSERT_TEXT and returns the SAFETY_VIOLATION envelope unwrapped', async () => {
		document.body.innerHTML = '<input id="q" type="text" value="">';
		const reply = (await call(watcher, {
			type: 'INSERT_TEXT',
			field_id: '#q',
			text: 'hi',
		})) as { kind: string };
		expect(reply.kind).toBe('InsertTextResult');
	});

	it('forwards INSERT_TEXT safety violations as Error envelopes without throwing', async () => {
		const reply = (await call(watcher, {
			type: 'INSERT_TEXT',
			field_id: '#nope',
			text: 'x',
		})) as { kind: string; code: string };
		expect(reply.kind).toBe('Error');
		expect(reply.code).toBe('SAFETY_VIOLATION');
	});

	it('wraps thrown handler errors in the Error envelope via guard()', async () => {
		// QUERY_SELECTOR with an invalid selector throws, which should be
		// caught by the guard and reshaped into `{kind: 'Error', data: …}`.
		const reply = (await call(watcher, { type: 'QUERY_SELECTOR', selector: '!!' })) as {
			kind: string;
			data: string;
		};
		expect(reply.kind).toBe('Error');
		expect(reply.data).toMatch(/not a valid CSS/);
	});
});

describe('Watcher subclass fall-through audit', () => {
	it('YoutubeWatcher.listen falls through to super.listen on unknown types', async () => {
		// Re-import the actual subclass and verify it forwards the new web
		// types to the base. We avoid pulling in the canvas / transcript
		// dependencies by stubbing them at module-mock time.
		vi.mock('../../../sites/youtube.com/transcript/index.js', () => ({
			YouTubeTranscriptApi: class {
				fetch = vi.fn();
			},
		}));
		const { YoutubeWatcher } =
			await import('../../../../../content/sites/youtube.com/index.js');
		const yt = new YoutubeWatcher({
			canvas: document.createElement('canvas'),
			youtubePlayer: null,
		});
		document.title = 'YT';
		const reply = (await yt.listen({ type: 'GET_PAGE_METADATA' }, SENDER)) as { kind: string };
		expect(reply.kind).toBe('PageMetadata');
	});
});
