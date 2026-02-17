import { sendMessageWithRetry } from './messaging';
import { getCurrentTabIcon } from './tabs';
import browser from 'webextension-polyfill';
import type { NativeMetadata, Frame } from '../content/bindings';

let collectionInterval: ReturnType<typeof setInterval> | null = null;
let activeNativePort: browser.Runtime.Port | null = null;
const lastUrl = new Map<number, string>();

export function initFocusTracker(port: browser.Runtime.Port): void {
	activeNativePort = port;

	browser.windows.onFocusChanged.addListener(onWindowFocusChanged);
	browser.tabs.onActivated.addListener(onTabActivated);
	browser.tabs.onUpdated.addListener(onTabUpdated);

	browser.windows
		.getLastFocused()
		.then((win) => {
			if (win && win.focused && win.id !== browser.windows.WINDOW_ID_NONE) {
				collectAndSend().catch(console.error);
				startCollectionInterval();
			}
		})
		.catch(console.error);
}

export function destroyFocusTracker(): void {
	stopCollectionInterval();
	activeNativePort = null;

	browser.windows.onFocusChanged.removeListener(onWindowFocusChanged);
	browser.tabs.onActivated.removeListener(onTabActivated);
	browser.tabs.onUpdated.removeListener(onTabUpdated);
}

export function setNativePort(port: browser.Runtime.Port | null): void {
	activeNativePort = port;
}

export async function onRemoved(tabId: number): Promise<void> {
	lastUrl.delete(tabId);
}

async function onWindowFocusChanged(windowId: number): Promise<void> {
	if (windowId === browser.windows.WINDOW_ID_NONE) {
		stopCollectionInterval();
	} else {
		await collectAndSend();
		startCollectionInterval();
	}
}

async function onTabActivated(_activeInfo: browser.Tabs.OnActivatedActiveInfoType): Promise<void> {
	if (!activeNativePort) return;
	await collectAndSend();
	startCollectionInterval();
}

async function onTabUpdated(
	_tabId: number,
	changeInfo: browser.Tabs.OnUpdatedChangeInfoType,
	tab: browser.Tabs.Tab,
): Promise<void> {
	if (changeInfo.status !== 'complete') return;
	if (!activeNativePort) return;
	if (!tab.active) return;
	await collectAndSend();
}

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

function isRealWebUrl(url: string): boolean {
	if (!url || typeof url !== 'string') return false;
	return /^https?:\/\//i.test(url);
}

async function collectAndSend(): Promise<void> {
	if (!activeNativePort) return;

	try {
		const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
		if (!activeTab || !activeTab.id || !activeTab.url || !isRealWebUrl(activeTab.url)) return;

		const port = activeNativePort;

		await sendMetadataEvent(activeTab, port);
		await sendAssetsEvent(activeTab.id, port);
		await sendSnapshotEvent(activeTab.id, port);
	} catch (error) {
		console.error('collectAndSend failed:', error);
	}
}

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
