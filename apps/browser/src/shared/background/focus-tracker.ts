import { sendMessageWithRetry } from './messaging';
import { getCurrentTabIcon } from './tabs';
import browser from 'webextension-polyfill';
import type { NativeMetadata, Frame } from '../content/bindings';

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

let collectionInterval: ReturnType<typeof setInterval> | null = null;
let isBrowserFocused = false;
let activeNativePort: browser.Runtime.Port | null = null;

/**
 * Track the last URL we sent metadata for so we avoid sending duplicate
 * TAB_ACTIVATED events when the domain hasn't changed (the Rust side also
 * deduplicates, but saving the round-trip is cheap).
 */
const lastUrl = new Map<number, string>();

// ---------------------------------------------------------------------------
// Public API – called from native-messenger.ts
// ---------------------------------------------------------------------------

/**
 * Wire up all listeners that drive the push-based collection model.
 * Must be called once after the native messaging port is connected.
 */
export function initFocusTracker(port: browser.Runtime.Port): void {
	activeNativePort = port;

	browser.windows.onFocusChanged.addListener(onWindowFocusChanged);
	browser.tabs.onActivated.addListener(onTabActivated);
	browser.tabs.onUpdated.addListener(onTabUpdated);

	// Check initial focus state – if the browser already has a focused window
	// when the extension loads we should start collection immediately.
	browser.windows
		.getLastFocused()
		.then((win) => {
			if (win && win.focused && win.id !== browser.windows.WINDOW_ID_NONE) {
				isBrowserFocused = true;
				collectAndSend().catch(console.error);
				startCollectionInterval();
			}
		})
		.catch(console.error);
}

/**
 * Tear down all listeners and stop any running interval.
 * Called when the native port disconnects.
 */
export function destroyFocusTracker(): void {
	stopCollectionInterval();
	isBrowserFocused = false;
	activeNativePort = null;

	browser.windows.onFocusChanged.removeListener(onWindowFocusChanged);
	browser.tabs.onActivated.removeListener(onTabActivated);
	browser.tabs.onUpdated.removeListener(onTabUpdated);
}

/**
 * Update the port reference (e.g. after a reconnect).
 */
export function setNativePort(port: browser.Runtime.Port | null): void {
	activeNativePort = port;
}

/**
 * Called when the native app detects that the browser window gained OS-level
 * focus.  This is more reliable than `windows.onFocusChanged` on some
 * platforms (notably Chrome on Linux) where the extension event can be missed
 * after returning from another application.
 */
export async function notifyBrowserFocused(): Promise<void> {
	if (isBrowserFocused) return; // already tracking
	isBrowserFocused = true;
	await collectAndSend();
	startCollectionInterval();
}

/**
 * Clean up URL tracking state when a tab is removed.
 */
export async function onRemoved(tabId: number): Promise<void> {
	lastUrl.delete(tabId);
}

// ---------------------------------------------------------------------------
// Window focus
// ---------------------------------------------------------------------------

async function onWindowFocusChanged(windowId: number): Promise<void> {
	if (windowId === browser.windows.WINDOW_ID_NONE) {
		// All browser windows lost focus (user switched to another app).
		isBrowserFocused = false;
		stopCollectionInterval();
	} else {
		// A browser window gained focus.
		isBrowserFocused = true;
		await collectAndSend();
		startCollectionInterval();
	}
}

// ---------------------------------------------------------------------------
// Tab events
//
// These events can only fire when the browser is in the foreground, so they
// double as an implicit focus-recovery mechanism for platforms where
// `windows.onFocusChanged` is unreliable (Chrome on Linux).
// ---------------------------------------------------------------------------

async function onTabActivated(_activeInfo: browser.Tabs.OnActivatedActiveInfoType): Promise<void> {
	if (!activeNativePort) return;
	if (!isBrowserFocused) {
		isBrowserFocused = true;
	}
	await collectAndSend();
	// Restart the interval so the next tick is a full 3 s from now.
	startCollectionInterval();
}

async function onTabUpdated(
	_tabId: number,
	changeInfo: browser.Tabs.OnUpdatedChangeInfoType,
	tab: browser.Tabs.Tab,
): Promise<void> {
	if (changeInfo.status !== 'complete') return;
	if (!activeNativePort) return;
	// Only care about the currently active tab.
	if (!tab.active) return;
	if (!isBrowserFocused) {
		isBrowserFocused = true;
		startCollectionInterval();
	}
	await collectAndSend();
}

// ---------------------------------------------------------------------------
// Collection interval
// ---------------------------------------------------------------------------

function startCollectionInterval(): void {
	stopCollectionInterval();
	collectionInterval = setInterval(() => {
		collectAndSend().catch(console.error);
	}, 3_000);
}

function stopCollectionInterval(): void {
	if (collectionInterval !== null) {
		clearInterval(collectionInterval);
		collectionInterval = null;
	}
}

// ---------------------------------------------------------------------------
// Core collection & send
// ---------------------------------------------------------------------------

function isRealWebUrl(url: string): boolean {
	if (!url || typeof url !== 'string') return false;
	return /^https?:\/\//i.test(url);
}

/**
 * Collect metadata, assets and snapshots from the active tab and push
 * them to the native app as Event frames on the native messaging port.
 */
async function collectAndSend(): Promise<void> {
	if (!activeNativePort) return;

	try {
		const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
		if (!activeTab || !activeTab.id || !activeTab.url || !isRealWebUrl(activeTab.url)) return;

		const port = activeNativePort; // capture in case it changes

		// 1. Metadata – always send (the Rust side de-duplicates by domain)
		await sendMetadataEvent(activeTab, port);

		// 2. Assets
		await sendAssetsEvent(activeTab.id, port);

		// 3. Snapshots
		await sendSnapshotEvent(activeTab.id, port);
	} catch (error) {
		console.error('collectAndSend failed:', error);
	}
}

// ---------------------------------------------------------------------------
// Event senders
// ---------------------------------------------------------------------------

async function sendMetadataEvent(tab: browser.Tabs.Tab, port: browser.Runtime.Port): Promise<void> {
	try {
		const iconBase64 = await getCurrentTabIcon(tab);

		const metadata = {
			kind: 'NativeMetadata',
			data: {
				url: tab.url,
				icon_base64: iconBase64,
			} as NativeMetadata,
		};

		const frame: Frame = {
			kind: {
				Event: {
					action: 'TAB_ACTIVATED',
					payload: JSON.stringify(metadata),
				},
			},
		};

		port.postMessage(frame);

		// Track last URL per tab (for legacy callers, mostly cosmetic).
		if (tab.id !== undefined && tab.url) {
			lastUrl.set(tab.id, tab.url);
		}
	} catch (error) {
		console.error('Failed to send metadata event:', error);
	}
}

async function sendAssetsEvent(tabId: number, port: browser.Runtime.Port): Promise<void> {
	try {
		const response = await sendMessageWithRetry(tabId, { type: 'GENERATE_ASSETS' });
		if (!response || response.kind === 'Error') return;

		const frame: Frame = {
			kind: {
				Event: {
					action: 'ASSETS',
					payload: JSON.stringify(response),
				},
			},
		};

		port.postMessage(frame);
	} catch (error) {
		// Content script may not be injected yet – expected for some pages.
		console.warn('Failed to collect assets:', error);
	}
}

async function sendSnapshotEvent(tabId: number, port: browser.Runtime.Port): Promise<void> {
	try {
		const response = await sendMessageWithRetry(tabId, { type: 'GENERATE_SNAPSHOT' });
		if (!response || response.kind === 'Error') return;

		const frame: Frame = {
			kind: {
				Event: {
					action: 'SNAPSHOT',
					payload: JSON.stringify(response),
				},
			},
		};

		port.postMessage(frame);
	} catch (error) {
		console.warn('Failed to collect snapshot:', error);
	}
}
