import { resolveFaviconBase64 } from './favicon';
import browser from 'webextension-polyfill';
import type { TabChange, TabStateBus } from './tab-state-bus';
import type { NativeMetadata, Frame, Payload } from '../content/bindings';

let activeNativePort: browser.Runtime.Port | null = null;
let unsubscribe: (() => void) | null = null;

export function initFocusTracker(port: browser.Runtime.Port, bus: TabStateBus): void {
	destroyFocusTracker();
	activeNativePort = port;
	unsubscribe = bus.subscribe(onTabChange);
}

export function destroyFocusTracker(): void {
	if (unsubscribe) {
		unsubscribe();
		unsubscribe = null;
	}
	activeNativePort = null;
}

export function setNativePort(port: browser.Runtime.Port | null): void {
	activeNativePort = port;
}

function isRealWebUrl(url: string): boolean {
	if (!url || typeof url !== 'string') return false;
	return /^https?:\/\//i.test(url);
}

async function onTabChange(change: TabChange): Promise<void> {
	if (!activeNativePort) return;

	// Mirror the previous filters: ignore window-focus-lost
	// (WINDOW_ID_NONE) transitions, ignore tab-removed entirely, and on
	// `updated` events only react when status completed or the title
	// changed.
	if (change.cause === 'removed') return;
	if (change.cause === 'window-focus' && change.windowId === -1) return;
	if (change.cause === 'updated') {
		const changeInfo = change.changeInfo ?? {};
		if (changeInfo.status !== 'complete' && !changeInfo.title) return;
		if (change.activeTab && !change.activeTab.active) return;
	}

	const tab = change.activeTab;
	if (!tab || tab.id === undefined || !tab.url || !isRealWebUrl(tab.url)) return;
	await sendMetadataEvent(
		tab as browser.Tabs.Tab & { id: number; url: string },
		activeNativePort,
	);
}

async function sendMetadataEvent(
	tab: browser.Tabs.Tab & { id: number; url: string },
	port: browser.Runtime.Port,
): Promise<void> {
	try {
		const iconBase64 = await resolveFaviconBase64(tab);

		const metadata = {
			kind: 'NativeMetadata',
			data: {
				// `tab_id` lets the desktop address this exact tab in
				// follow-up `LIST_TOOLS` / `INVOKE_TOOL` calls without
				// the extension having to re-query `chrome.tabs`. Always
				// present — the guard in `onTabChange` rejects tabs
				// without an id before we get here.
				tab_id: tab.id,
				url: tab.url,
				icon_base64: iconBase64,
				title: tab.title ?? null,
			} as NativeMetadata,
		};

		const frame: Frame = {
			kind: {
				Event: {
					action: 'TAB_ACTIVATED',
					// Inline JSON payload — the bridge protocol decodes it
					// as a typed value at the Rust boundary, no double
					// `JSON.parse` required.
					payload: metadata as Payload,
				},
			},
		};

		port.postMessage(frame);
	} catch (error) {
		console.error('Failed to send metadata event:', error);
	}
}
