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

describe('context-observer transitions', () => {
	it('activates with envelope-stamped payload when the active tab is a watch page', async () => {
		const port = fakePort();
		const { bus } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

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
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();
		expect(postedFrames(port)).toHaveLength(1);

		// Same tab activated again; transition() should observe no change.
		await emit({ cause: 'activated', activeTab: watchTab() });
		expect(postedFrames(port)).toHaveLength(1);
	});

	it('emits deactivate then activate when the same tab navigates to a new video', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();
		expect(postedFrames(port)).toHaveLength(1);

		await emit({
			cause: 'updated',
			activeTab: watchTab({
				url: 'https://www.youtube.com/watch?v=def456',
				title: 'Another video',
			}),
			changeInfo: { url: 'https://www.youtube.com/watch?v=def456' },
		});

		const events = postedFrames(port).map(frameEvent);
		expect(events.map((e) => e.action)).toEqual([
			'CONTEXT_ACTIVATED',
			'CONTEXT_DEACTIVATED',
			'CONTEXT_ACTIVATED',
		]);
		expect((events[1].payload as { key: string }).key).toBe('youtube::watch_page');
		expect((events[2].payload as { data: { video_id: string } }).data.video_id).toBe('def456');
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

		const events = postedFrames(port).map(frameEvent);
		expect(events.at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
	});

	it('deactivates on window-focus-lost (WINDOW_ID_NONE)', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		await emit({ cause: 'window-focus', windowId: -1, activeTab: watchTab() });

		const events = postedFrames(port).map(frameEvent);
		expect(events.at(-1)?.action).toBe('CONTEXT_DEACTIVATED');
	});

	it('deactivates when the active tab is removed', async () => {
		const port = fakePort();
		const { bus, emit } = makeBus(watchTab());
		observer.startContextObserver(port, bus);
		await flushMicrotasks();
		expect(postedFrames(port)).toHaveLength(1);

		await emit({ cause: 'removed', removedTabId: 19, activeTab: undefined });

		const events = postedFrames(port).map(frameEvent);
		expect(events).toHaveLength(2);
		expect(events[1].action).toBe('CONTEXT_DEACTIVATED');
	});

	it('does not emit anything when starting on a non-watch tab', async () => {
		const port = fakePort();
		const { bus } = makeBus(watchTab({ url: 'https://www.example.com/' }));
		observer.startContextObserver(port, bus);
		await flushMicrotasks();

		expect(postedFrames(port)).toHaveLength(0);
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
