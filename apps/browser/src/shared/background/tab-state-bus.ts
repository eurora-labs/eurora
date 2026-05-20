import browser from 'webextension-polyfill';

/// Why a [`TabChange`] was emitted. Subscribers can branch on this when
/// the active-tab object alone isn't enough — for instance, the focus
/// tracker debounces by `updated` events that lack a `title` change.
export type TabChangeCause = 'activated' | 'updated' | 'removed' | 'window-focus' | 'initial-sync';

/// One observed tab transition. `activeTab` is whatever
/// `browser.tabs.query({ active: true, currentWindow: true })` returned
/// at the moment the event fired — including `undefined` when no window
/// is focused (e.g. `WINDOW_ID_NONE`).
///
/// The bus performs the `tabs.query` exactly once per event so every
/// subscriber sees the same active-tab snapshot. Subscribers must not
/// rely on `activeTab` outliving the handler call: re-query if you need
/// fresh state outside the synchronous handler scope.
export interface TabChange {
	cause: TabChangeCause;
	activeTab: browser.Tabs.Tab | undefined;
	/// Only set when `cause === 'updated'`. The raw `changeInfo` from
	/// `chrome.tabs.onUpdated` so subscribers can debounce on specific
	/// fields (status complete, url, title…).
	changeInfo?: browser.Tabs.OnUpdatedChangeInfoType;
	/// Only set when `cause === 'removed'`.
	removedTabId?: number;
	/// Only set when `cause === 'window-focus'`. `browser.windows.WINDOW_ID_NONE`
	/// when the browser lost focus entirely.
	windowId?: number;
}

export type TabChangeHandler = (change: TabChange) => void | Promise<void>;

/// Single-subscription point for the browser-tab event stream that the
/// context observer and focus tracker both react to. Replaces the prior
/// pattern where each consumer registered its own
/// `tabs.{onActivated,onUpdated,onRemoved}` + `windows.onFocusChanged`
/// listeners and ran its own `tabs.query`.
///
/// Construct once per native-port lifecycle, hand the resulting bus to
/// every subscriber, and call [`TabStateBus.stop`] before reconnect.
export interface TabStateBus {
	/// Register a handler. The bus immediately delivers an
	/// `initial-sync` change so the subscriber sees the current active
	/// tab without racing the first user-driven event. Returns an
	/// unsubscribe function.
	subscribe(handler: TabChangeHandler): () => void;
	/// Detach every browser listener and drop all subscribers. After
	/// `stop()` the bus is inert; subscribers attached afterwards never
	/// fire.
	stop(): void;
}

/// Construct and start the bus. Registers exactly one set of browser
/// listeners; every subscriber sees every event.
///
/// The bus calls subscribers in registration order. A throwing
/// subscriber does not prevent other subscribers from running — errors
/// are logged and swallowed.
export function startTabStateBus(): TabStateBus {
	const handlers: TabChangeHandler[] = [];
	let active = true;

	async function queryActiveTab(): Promise<browser.Tabs.Tab | undefined> {
		try {
			const tabs = await browser.tabs.query({ active: true, currentWindow: true });
			return tabs[0];
		} catch (err) {
			console.error('tab-state-bus: failed to query active tab:', err);
			return undefined;
		}
	}

	async function notify(targets: TabChangeHandler[], change: TabChange): Promise<void> {
		for (const handler of targets) {
			try {
				await handler(change);
			} catch (err) {
				console.error('tab-state-bus subscriber threw:', err);
			}
		}
	}

	async function dispatchAll(
		cause: TabChangeCause,
		extras: Partial<TabChange> = {},
	): Promise<void> {
		if (!active) return;
		const activeTab = await queryActiveTab();
		const change: TabChange = { cause, activeTab, ...extras };
		await notify(handlers.slice(), change);
	}

	async function dispatchOne(
		handler: TabChangeHandler,
		cause: TabChangeCause,
		extras: Partial<TabChange> = {},
	): Promise<void> {
		if (!active) return;
		const activeTab = await queryActiveTab();
		await notify([handler], { cause, activeTab, ...extras });
	}

	function onActivated(_info: browser.Tabs.OnActivatedActiveInfoType): void {
		void dispatchAll('activated');
	}

	function onUpdated(
		_tabId: number,
		changeInfo: browser.Tabs.OnUpdatedChangeInfoType,
		_tab: browser.Tabs.Tab,
	): void {
		void dispatchAll('updated', { changeInfo });
	}

	function onRemoved(tabId: number): void {
		void dispatchAll('removed', { removedTabId: tabId });
	}

	function onWindowFocusChanged(windowId: number): void {
		void dispatchAll('window-focus', { windowId });
	}

	browser.tabs.onActivated.addListener(onActivated);
	browser.tabs.onUpdated.addListener(onUpdated);
	browser.tabs.onRemoved.addListener(onRemoved);
	browser.windows.onFocusChanged.addListener(onWindowFocusChanged);

	return {
		subscribe(handler: TabChangeHandler): () => void {
			handlers.push(handler);
			// Only the new subscriber gets the initial sync — already
			// attached subscribers saw their own when they subscribed.
			void dispatchOne(handler, 'initial-sync');
			return () => {
				const idx = handlers.indexOf(handler);
				if (idx >= 0) handlers.splice(idx, 1);
			};
		},
		stop(): void {
			active = false;
			handlers.length = 0;
			browser.tabs.onActivated.removeListener(onActivated);
			browser.tabs.onUpdated.removeListener(onUpdated);
			browser.tabs.onRemoved.removeListener(onRemoved);
			browser.windows.onFocusChanged.removeListener(onWindowFocusChanged);
		},
	};
}
