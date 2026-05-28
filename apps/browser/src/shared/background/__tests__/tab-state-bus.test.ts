import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { TabChange } from '../tab-state-bus';
import type browser from 'webextension-polyfill';

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

let mod: typeof import('../tab-state-bus');
let bus: ReturnType<typeof import('../tab-state-bus').startTabStateBus>;

beforeEach(async () => {
	for (const key of Object.keys(listeners) as (keyof typeof listeners)[]) {
		listeners[key].length = 0;
	}
	tabsQueryMock.mockReset();
	vi.resetModules();
	mod = await import('../tab-state-bus');
});

afterEach(() => {
	bus?.stop();
});

function fakeTab(overrides: Partial<browser.Tabs.Tab> = {}): browser.Tabs.Tab {
	return {
		id: 19,
		windowId: 7,
		url: 'https://www.example.com/',
		title: 'Example',
		active: true,
		...overrides,
	} as browser.Tabs.Tab;
}

async function flush(): Promise<void> {
	// Two microtasks: one for the query promise, one for the
	// dispatch promise that awaits it.
	await Promise.resolve();
	await Promise.resolve();
	await Promise.resolve();
}

describe('startTabStateBus', () => {
	it('registers exactly one listener per browser event', async () => {
		tabsQueryMock.mockResolvedValue([fakeTab()]);
		bus = mod.startTabStateBus();
		expect(listeners.tabsOnActivated).toHaveLength(1);
		expect(listeners.tabsOnUpdated).toHaveLength(1);
		expect(listeners.tabsOnRemoved).toHaveLength(1);
		expect(listeners.windowsOnFocusChanged).toHaveLength(1);
	});

	it('emits initial-sync exactly once to a new subscriber and not to existing ones', async () => {
		tabsQueryMock.mockResolvedValue([fakeTab()]);
		bus = mod.startTabStateBus();

		const a = vi.fn();
		bus.subscribe(a);
		await flush();
		expect(a).toHaveBeenCalledTimes(1);
		expect(a.mock.calls[0][0].cause).toBe('initial-sync');

		const b = vi.fn();
		bus.subscribe(b);
		await flush();
		// Only b receives the second initial-sync.
		expect(b).toHaveBeenCalledTimes(1);
		expect(b.mock.calls[0][0].cause).toBe('initial-sync');
		expect(a).toHaveBeenCalledTimes(1);
	});

	it('fans every user-driven event out to every subscriber with the same snapshot', async () => {
		tabsQueryMock.mockResolvedValue([fakeTab()]);
		bus = mod.startTabStateBus();

		const a = vi.fn();
		const b = vi.fn();
		bus.subscribe(a);
		bus.subscribe(b);
		await flush();

		// Both saw their own initial-sync.
		expect(a).toHaveBeenCalledTimes(1);
		expect(b).toHaveBeenCalledTimes(1);

		tabsQueryMock.mockResolvedValue([fakeTab({ id: 20 })]);
		await listeners.tabsOnActivated[0]({ tabId: 20, windowId: 7 });
		await flush();

		expect(a).toHaveBeenCalledTimes(2);
		expect(b).toHaveBeenCalledTimes(2);
		const lastA = a.mock.calls[1][0] as TabChange;
		const lastB = b.mock.calls[1][0] as TabChange;
		expect(lastA.cause).toBe('activated');
		expect(lastB.cause).toBe('activated');
		expect(lastA.activeTab?.id).toBe(20);
		expect(lastB.activeTab?.id).toBe(20);
	});

	it('forwards changeInfo on updated events', async () => {
		tabsQueryMock.mockResolvedValue([fakeTab()]);
		bus = mod.startTabStateBus();
		const sub = vi.fn();
		bus.subscribe(sub);
		await flush();

		tabsQueryMock.mockResolvedValue([fakeTab({ url: 'https://www.example.org/' })]);
		await listeners.tabsOnUpdated[0](
			19,
			{ url: 'https://www.example.org/' },
			fakeTab({ url: 'https://www.example.org/' }),
		);
		await flush();

		const change = sub.mock.calls.at(-1)?.[0] as TabChange;
		expect(change.cause).toBe('updated');
		expect(change.changeInfo).toEqual({ url: 'https://www.example.org/' });
	});

	it('forwards removedTabId on removed events', async () => {
		tabsQueryMock.mockResolvedValue([]);
		bus = mod.startTabStateBus();
		const sub = vi.fn();
		bus.subscribe(sub);
		await flush();

		listeners.tabsOnRemoved[0](19);
		await flush();

		const change = sub.mock.calls.at(-1)?.[0] as TabChange;
		expect(change.cause).toBe('removed');
		expect(change.removedTabId).toBe(19);
	});

	it('forwards windowId on window-focus events', async () => {
		tabsQueryMock.mockResolvedValue([]);
		bus = mod.startTabStateBus();
		const sub = vi.fn();
		bus.subscribe(sub);
		await flush();

		await listeners.windowsOnFocusChanged[0](browserMock.windows.WINDOW_ID_NONE);
		await flush();

		const change = sub.mock.calls.at(-1)?.[0] as TabChange;
		expect(change.cause).toBe('window-focus');
		expect(change.windowId).toBe(browserMock.windows.WINDOW_ID_NONE);
	});

	it('isolates throwing subscribers from siblings', async () => {
		tabsQueryMock.mockResolvedValue([fakeTab()]);
		bus = mod.startTabStateBus();
		const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

		const throwing = vi.fn(() => {
			throw new Error('boom');
		});
		const sibling = vi.fn();
		bus.subscribe(throwing);
		bus.subscribe(sibling);
		await flush();

		// Both received their own initial-sync; the throw didn't stop the sibling.
		expect(throwing).toHaveBeenCalled();
		expect(sibling).toHaveBeenCalled();
		expect(errorSpy).toHaveBeenCalled();
		errorSpy.mockRestore();
	});

	it('unsubscribe stops further deliveries to that handler', async () => {
		tabsQueryMock.mockResolvedValue([fakeTab()]);
		bus = mod.startTabStateBus();
		const sub = vi.fn();
		const off = bus.subscribe(sub);
		await flush();
		expect(sub).toHaveBeenCalledTimes(1);

		off();
		await listeners.tabsOnActivated[0]({ tabId: 19, windowId: 7 });
		await flush();
		expect(sub).toHaveBeenCalledTimes(1);
	});

	it('stop() removes every browser listener', async () => {
		tabsQueryMock.mockResolvedValue([fakeTab()]);
		bus = mod.startTabStateBus();
		bus.stop();
		expect(listeners.tabsOnActivated).toHaveLength(0);
		expect(listeners.tabsOnUpdated).toHaveLength(0);
		expect(listeners.tabsOnRemoved).toHaveLength(0);
		expect(listeners.windowsOnFocusChanged).toHaveLength(0);
	});

	it('subscribers attached after stop() never fire', async () => {
		tabsQueryMock.mockResolvedValue([fakeTab()]);
		bus = mod.startTabStateBus();
		bus.stop();

		const sub = vi.fn();
		bus.subscribe(sub);
		await flush();
		expect(sub).not.toHaveBeenCalled();
	});
});
