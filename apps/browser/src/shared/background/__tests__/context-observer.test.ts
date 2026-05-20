import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { Frame } from '../../content/bindings';
import type browser from 'webextension-polyfill';

// Listener registry the mocked `browser` exposes. Tests invoke these
// directly to simulate user actions without depending on the
// webextension event loop.
const listeners = {
	tabsOnActivated: [] as Array<(info: { tabId: number; windowId: number }) => unknown>,
	tabsOnUpdated: [] as Array<
		(tabId: number, changeInfo: Record<string, unknown>, tab: unknown) => unknown
	>,
	tabsOnRemoved: [] as Array<(tabId: number) => unknown>,
	windowsOnFocusChanged: [] as Array<(windowId: number) => unknown>,
};

function makeListener<T extends keyof typeof listeners>(name: T) {
	return {
		addListener: vi.fn((fn: (typeof listeners)[T][number]) => {
			listeners[name].push(fn as never);
		}),
		removeListener: vi.fn((fn: (typeof listeners)[T][number]) => {
			const idx = listeners[name].indexOf(fn as never);
			if (idx >= 0) listeners[name].splice(idx, 1);
		}),
	};
}

const tabsQueryMock = vi.fn();
const browserMock = {
	tabs: {
		onActivated: makeListener('tabsOnActivated'),
		onUpdated: makeListener('tabsOnUpdated'),
		onRemoved: makeListener('tabsOnRemoved'),
		query: tabsQueryMock,
	},
	windows: {
		onFocusChanged: makeListener('windowsOnFocusChanged'),
		WINDOW_ID_NONE: -1,
	},
};

vi.mock('webextension-polyfill', () => ({ default: browserMock }));

let observer: typeof import('../context-observer');

beforeEach(async () => {
	for (const key of Object.keys(listeners) as (keyof typeof listeners)[]) {
		listeners[key].length = 0;
	}
	tabsQueryMock.mockReset();
	vi.resetModules();
	observer = await import('../context-observer');
});

afterEach(() => {
	observer.stopContextObserver();
});

type FakePort = browser.Runtime.Port & { postMessage: ReturnType<typeof vi.fn> };

function fakePort(): FakePort {
	return { postMessage: vi.fn() } as unknown as FakePort;
}

function watchTab(overrides: Partial<browser.Tabs.Tab> = {}): browser.Tabs.Tab {
	return {
		id: 19,
		windowId: 7,
		url: 'https://www.youtube.com/watch?v=abc123',
		title: 'Tokio async patterns',
		active: true,
		...overrides,
	} as browser.Tabs.Tab;
}

function postedFrames(port: FakePort): Frame[] {
	return port.postMessage.mock.calls.map((c) => c[0] as Frame);
}

function frameEvent(frame: Frame): { action: string; payload: unknown } {
	const kind = frame.kind as { Event?: { action: string; payload?: unknown } };
	if (!kind.Event) throw new Error('expected Event frame');
	return {
		action: kind.Event.action,
		// The bridge payload is inline JSON — already a JS value, no
		// `JSON.parse` step.
		payload: kind.Event.payload ?? null,
	};
}

describe('classifyYoutubeUrl', () => {
	it('matches www.youtube.com/watch?v=…', () => {
		expect(observer.classifyYoutubeUrl('https://www.youtube.com/watch?v=abc')).toEqual({
			videoId: 'abc',
			pageUrl: 'https://www.youtube.com/watch?v=abc',
		});
	});

	it('matches m.youtube.com and bare youtube.com', () => {
		expect(observer.classifyYoutubeUrl('https://m.youtube.com/watch?v=xyz')).not.toBeNull();
		expect(observer.classifyYoutubeUrl('https://youtube.com/watch?v=xyz')).not.toBeNull();
	});

	it('returns null for non-watch YouTube pages', () => {
		expect(observer.classifyYoutubeUrl('https://www.youtube.com/')).toBeNull();
		expect(observer.classifyYoutubeUrl('https://www.youtube.com/feed/trending')).toBeNull();
	});

	it('returns null for unrelated hosts', () => {
		expect(observer.classifyYoutubeUrl('https://vimeo.com/watch?v=abc')).toBeNull();
	});

	it('returns null for /watch without a v parameter', () => {
		expect(observer.classifyYoutubeUrl('https://www.youtube.com/watch')).toBeNull();
	});

	it('returns null for invalid URL strings', () => {
		expect(observer.classifyYoutubeUrl('not a url')).toBeNull();
	});
});

describe('context-observer transitions', () => {
	it('activates with envelope-stamped payload when the active tab is a watch page', async () => {
		tabsQueryMock.mockResolvedValueOnce([watchTab()]);
		const port = fakePort();
		observer.startContextObserver(port);
		await Promise.resolve();
		await Promise.resolve();

		const frames = postedFrames(port);
		expect(frames).toHaveLength(1);
		const evt = frameEvent(frames[0]);
		expect(evt.action).toBe('CONTEXT_ACTIVATED');
		expect(evt.payload).toEqual({
			key: 'youtube::watch_page',
			data: {
				video_id: 'abc123',
				title: 'Tokio async patterns',
				page_url: 'https://www.youtube.com/watch?v=abc123',
			},
			origin: {
				Browser: {
					tab_id: 19,
					window_id: 'win-7',
					page_url: 'https://www.youtube.com/watch?v=abc123',
				},
			},
		});
	});

	it('does not republish on a no-op resync of the same tab and video', async () => {
		tabsQueryMock.mockResolvedValue([watchTab()]);
		const port = fakePort();
		observer.startContextObserver(port);
		await Promise.resolve();
		await Promise.resolve();

		expect(postedFrames(port)).toHaveLength(1);

		// Fire a tab-activated event for the same tab; the resync should
		// see no change and not emit a second frame.
		await listeners.tabsOnActivated[0]({ tabId: 19, windowId: 7 });
		expect(postedFrames(port)).toHaveLength(1);
	});

	it('emits deactivate then activate when the same tab navigates to a new video', async () => {
		const port = fakePort();
		tabsQueryMock.mockResolvedValueOnce([watchTab()]);
		observer.startContextObserver(port);
		await Promise.resolve();
		await Promise.resolve();
		expect(postedFrames(port)).toHaveLength(1);

		tabsQueryMock.mockResolvedValueOnce([
			watchTab({ url: 'https://www.youtube.com/watch?v=def456', title: 'Another video' }),
		]);
		await listeners.tabsOnUpdated[0](
			19,
			{ url: 'https://www.youtube.com/watch?v=def456' },
			watchTab({ url: 'https://www.youtube.com/watch?v=def456' }),
		);

		const events = postedFrames(port).map(frameEvent);
		expect(events.map((e) => e.action)).toEqual([
			'CONTEXT_ACTIVATED',
			'CONTEXT_DEACTIVATED',
			'CONTEXT_ACTIVATED',
		]);
		// The deactivate carries the *previous* key — not video-scoped.
		expect((events[1].payload as { key: string }).key).toBe('youtube::watch_page');
		expect((events[2].payload as { data: { video_id: string } }).data.video_id).toBe('def456');
	});

	it('deactivates on switch to a non-watch page', async () => {
		const port = fakePort();
		tabsQueryMock.mockResolvedValueOnce([watchTab()]);
		observer.startContextObserver(port);
		await Promise.resolve();
		await Promise.resolve();

		tabsQueryMock.mockResolvedValueOnce([
			watchTab({ id: 20, url: 'https://www.example.com/' }),
		]);
		await listeners.tabsOnActivated[0]({ tabId: 20, windowId: 7 });

		const events = postedFrames(port).map(frameEvent);
		expect(events.at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
	});

	it('ignores window-focus-lost transitions (WINDOW_ID_NONE) by deactivating', async () => {
		const port = fakePort();
		tabsQueryMock.mockResolvedValueOnce([watchTab()]);
		observer.startContextObserver(port);
		await Promise.resolve();
		await Promise.resolve();

		await listeners.windowsOnFocusChanged[0](browserMock.windows.WINDOW_ID_NONE);

		const events = postedFrames(port).map(frameEvent);
		expect(events.at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
	});

	it('deactivates when the active tab is removed', async () => {
		const port = fakePort();
		tabsQueryMock.mockResolvedValueOnce([watchTab()]);
		observer.startContextObserver(port);
		await Promise.resolve();
		await Promise.resolve();
		expect(postedFrames(port)).toHaveLength(1);

		listeners.tabsOnRemoved[0](19);

		const events = postedFrames(port).map(frameEvent);
		expect(events).toHaveLength(2);
		expect(events[1].action).toBe('CONTEXT_DEACTIVATED');
	});

	it('does not emit anything when starting on a non-watch tab', async () => {
		tabsQueryMock.mockResolvedValueOnce([watchTab({ url: 'https://www.example.com/' })]);
		const port = fakePort();
		observer.startContextObserver(port);
		await Promise.resolve();
		await Promise.resolve();

		expect(postedFrames(port)).toHaveLength(0);
	});

	it('ignores tab-updated events that did not change url or status', async () => {
		const port = fakePort();
		tabsQueryMock.mockResolvedValueOnce([watchTab()]);
		observer.startContextObserver(port);
		await Promise.resolve();
		await Promise.resolve();
		const baseline = postedFrames(port).length;

		await listeners.tabsOnUpdated[0](
			19,
			{ favIconUrl: 'https://www.youtube.com/favicon.ico' },
			watchTab(),
		);

		expect(postedFrames(port).length).toBe(baseline);
	});
});
