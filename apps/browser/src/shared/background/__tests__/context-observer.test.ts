import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { Frame } from '../../content/bindings';
import type { TabChange, TabStateBus } from '../tab-state-bus';
import type browser from 'webextension-polyfill';

vi.mock('webextension-polyfill', () => ({
	default: {
		// The observer only touches `webextension-polyfill` for type
		// imports now; the bus owns the runtime side. A stub is enough
		// to keep `await import` happy.
		tabs: {},
		windows: { WINDOW_ID_NONE: -1 },
		runtime: {},
	},
}));

let observer: typeof import('../context-observer');

beforeEach(async () => {
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

/// Test bus that lets the test driver push synthetic [`TabChange`]
/// events at the subscriber. Mirrors the production [`TabStateBus`] API
/// — `subscribe` registers the handler and immediately emits an
/// `initial-sync` change carrying the supplied tab snapshot.
function makeBus(initial: browser.Tabs.Tab | undefined): {
	bus: TabStateBus;
	emit: (change: TabChange) => Promise<void>;
} {
	const handlers: ((change: TabChange) => void | Promise<void>)[] = [];
	let currentTab = initial;
	return {
		bus: {
			subscribe(handler) {
				handlers.push(handler);
				// Synchronous-ish microtask: matches the bus's
				// `dispatchOne` which awaits its `queryActiveTab` promise
				// before invoking the handler.
				void Promise.resolve().then(
					async () => await handler({ cause: 'initial-sync', activeTab: currentTab }),
				);
				return () => {
					const idx = handlers.indexOf(handler);
					if (idx >= 0) handlers.splice(idx, 1);
				};
			},
			stop() {
				handlers.length = 0;
			},
		},
		async emit(change) {
			currentTab = change.activeTab ?? currentTab;
			for (const h of handlers.slice()) {
				await h(change);
			}
		},
	};
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

function plainTab(overrides: Partial<browser.Tabs.Tab> = {}): browser.Tabs.Tab {
	return {
		id: 30,
		windowId: 7,
		url: 'https://example.com/article',
		title: 'Example article',
		active: true,
		...overrides,
	} as browser.Tabs.Tab;
}

function postedFrames(port: FakePort): Frame[] {
	return port.postMessage.mock.calls.map((c) => c[0] as Frame);
}

function frameEvent(frame: Frame): {
	action: string;
	payload: { key: string; data?: unknown; tab_id?: number };
} {
	const kind = frame.kind as { Event?: { action: string; payload?: unknown } };
	if (!kind.Event) throw new Error('expected Event frame');
	return {
		action: kind.Event.action,
		payload: kind.Event.payload as { key: string; data?: unknown; tab_id?: number },
	};
}

function eventsFor(port: FakePort, key: string): { action: string; payload: unknown }[] {
	return postedFrames(port)
		.map(frameEvent)
		.filter((e) => e.payload.key === key);
}

async function flushMicrotasks(): Promise<void> {
	await Promise.resolve();
	await Promise.resolve();
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

describe('classifyWebUrl', () => {
	it('matches http and https URLs', () => {
		expect(observer.classifyWebUrl('https://example.com/path')).toEqual({
			pageUrl: 'https://example.com/path',
			host: 'example.com',
		});
		expect(observer.classifyWebUrl('http://example.com/')).not.toBeNull();
	});

	it('rejects non-http(s) schemes', () => {
		expect(observer.classifyWebUrl('about:blank')).toBeNull();
		expect(observer.classifyWebUrl('chrome://settings/')).toBeNull();
		expect(observer.classifyWebUrl('chrome-extension://abc/popup.html')).toBeNull();
		expect(observer.classifyWebUrl('file:///etc/hosts')).toBeNull();
		expect(observer.classifyWebUrl('data:text/html,<h1>x</h1>')).toBeNull();
		expect(observer.classifyWebUrl('javascript:void(0)')).toBeNull();
		expect(observer.classifyWebUrl('view-source:https://example.com')).toBeNull();
	});

	it('returns null for invalid URL strings', () => {
		expect(observer.classifyWebUrl('not a url')).toBeNull();
	});
});

describe('youtube::watch_page transitions', () => {
	it('activates with envelope-stamped payload when the active tab is a watch page', async () => {
		const port = fakePort();
		const { bus } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		const events = eventsFor(port, 'youtube::watch_page');
		expect(events).toHaveLength(1);
		expect(events[0]).toEqual({
			action: 'CONTEXT_ACTIVATED',
			payload: {
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
			},
		});
	});

	it('does not republish on a no-op resync of the same tab and video', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();
		const baseline = eventsFor(port, 'youtube::watch_page').length;

		await emit({ cause: 'activated', activeTab: watchTab() });
		expect(eventsFor(port, 'youtube::watch_page')).toHaveLength(baseline);
	});

	it('emits deactivate then activate when the same tab navigates to a new video', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		await emit({
			cause: 'updated',
			activeTab: watchTab({
				url: 'https://www.youtube.com/watch?v=def456',
				title: 'Another video',
			}),
			changeInfo: { url: 'https://www.youtube.com/watch?v=def456' },
		});

		const events = eventsFor(port, 'youtube::watch_page');
		expect(events.map((e) => e.action)).toEqual([
			'CONTEXT_ACTIVATED',
			'CONTEXT_DEACTIVATED',
			'CONTEXT_ACTIVATED',
		]);
	});

	it('deactivates on switch to a non-watch page', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		await emit({
			cause: 'activated',
			activeTab: watchTab({ id: 20, url: 'https://www.example.com/' }),
		});

		const ytEvents = eventsFor(port, 'youtube::watch_page');
		expect(ytEvents.at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
	});

	it('deactivates when the active tab is removed', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		await emit({ cause: 'removed', removedTabId: 19, activeTab: undefined });

		const ytEvents = eventsFor(port, 'youtube::watch_page');
		expect(ytEvents.at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
	});

	it('ignores tab-updated events that did not change url or status', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();
		const baseline = postedFrames(port).length;

		await emit({
			cause: 'updated',
			activeTab: watchTab(),
			changeInfo: { favIconUrl: 'https://www.youtube.com/favicon.ico' },
		});

		expect(postedFrames(port).length).toBe(baseline);
	});
});

describe('web::page transitions', () => {
	it('activates on a plain http(s) tab', async () => {
		const port = fakePort();
		const { bus } = makeBus(plainTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		const events = eventsFor(port, 'web::page');
		expect(events).toHaveLength(1);
		expect(events[0].action).toBe('CONTEXT_ACTIVATED');
		const payload = events[0].payload as {
			data: { url: string; host: string; title: string | null; language: string | null };
			origin: { Browser: { tab_id: number; window_id: string; page_url: string } };
		};
		expect(payload.data.url).toBe('https://example.com/article');
		expect(payload.data.host).toBe('example.com');
		expect(payload.data.title).toBe('Example article');
		expect(payload.data.language).toBeNull();
		expect(payload.origin.Browser.tab_id).toBe(30);
	});

	it('deactivates on switch to about:blank', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(plainTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		await emit({
			cause: 'activated',
			activeTab: plainTab({ url: 'about:blank' }),
		});

		const events = eventsFor(port, 'web::page');
		expect(events.at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
	});

	it('does not activate on chrome:// or extension URLs', async () => {
		const port = fakePort();
		const { bus } = makeBus(plainTab({ url: 'chrome://settings/' }));
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		expect(eventsFor(port, 'web::page')).toHaveLength(0);
	});

	it('deactivates on window-focus-lost (WINDOW_ID_NONE)', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(plainTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		await emit({ cause: 'window-focus', windowId: -1, activeTab: plainTab() });

		const events = eventsFor(port, 'web::page');
		expect(events.at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
	});

	it('deactivates when the active tab is removed', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(plainTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		await emit({ cause: 'removed', removedTabId: 30, activeTab: undefined });

		const events = eventsFor(port, 'web::page');
		expect(events.at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
	});

	it('re-activates on URL change within the same tab', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(plainTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		await emit({
			cause: 'updated',
			activeTab: plainTab({ url: 'https://example.com/other' }),
			changeInfo: { url: 'https://example.com/other' },
		});

		const events = eventsFor(port, 'web::page');
		expect(events.map((e) => e.action)).toEqual([
			'CONTEXT_ACTIVATED',
			'CONTEXT_DEACTIVATED',
			'CONTEXT_ACTIVATED',
		]);
	});
});

describe('coexistence: web::page and youtube::watch_page', () => {
	it('emits both activations when the active tab is a YouTube watch page', async () => {
		const port = fakePort();
		const { bus } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		const youtubeEvents = eventsFor(port, 'youtube::watch_page');
		const webEvents = eventsFor(port, 'web::page');
		expect(youtubeEvents.map((e) => e.action)).toEqual(['CONTEXT_ACTIVATED']);
		expect(webEvents.map((e) => e.action)).toEqual(['CONTEXT_ACTIVATED']);
	});

	it('deactivates only youtube::watch_page when navigating from watch to homepage', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		await emit({
			cause: 'updated',
			activeTab: watchTab({ url: 'https://www.youtube.com/' }),
			changeInfo: { url: 'https://www.youtube.com/' },
		});

		const youtubeEvents = eventsFor(port, 'youtube::watch_page');
		const webEvents = eventsFor(port, 'web::page');
		expect(youtubeEvents.map((e) => e.action)).toEqual([
			'CONTEXT_ACTIVATED',
			'CONTEXT_DEACTIVATED',
		]);
		// `web::page` swaps because the URL changed, but it stays active
		// throughout — the model still has access to generic web tools.
		expect(webEvents.map((e) => e.action)).toEqual([
			'CONTEXT_ACTIVATED',
			'CONTEXT_DEACTIVATED',
			'CONTEXT_ACTIVATED',
		]);
	});

	it('deactivates every key on window-focus-lost', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		await emit({ cause: 'window-focus', windowId: -1, activeTab: watchTab() });

		expect(eventsFor(port, 'youtube::watch_page').at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
		expect(eventsFor(port, 'web::page').at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
	});

	it('does not emit either when starting on chrome:// or about:blank', async () => {
		const port = fakePort();
		const { bus } = makeBus(plainTab({ url: 'about:blank' }));
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		expect(postedFrames(port)).toHaveLength(0);
	});
});
