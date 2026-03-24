import { getCurrentTabIcon } from './tabs';
import browser from 'webextension-polyfill';
import type { NativeMetadata, Frame } from '../content/bindings';

let activeNativePort: browser.Runtime.Port | null = null;

export function initFocusTracker(port: browser.Runtime.Port): void {
	activeNativePort = port;

	browser.tabs.onActivated.addListener(onTabActivated);
	browser.tabs.onUpdated.addListener(onTabUpdated);
	browser.tabs.onRemoved.addListener(onTabRemoved);
	browser.windows.onFocusChanged.addListener(onWindowFocusChanged);

	sendMetadataForActiveTab().catch(console.error);
}

export function destroyFocusTracker(): void {
	activeNativePort = null;

	browser.tabs.onActivated.removeListener(onTabActivated);
	browser.tabs.onUpdated.removeListener(onTabUpdated);
	browser.tabs.onRemoved.removeListener(onTabRemoved);
	browser.windows.onFocusChanged.removeListener(onWindowFocusChanged);
}

export function setNativePort(port: browser.Runtime.Port | null): void {
	activeNativePort = port;
}

function onTabRemoved(_tabId: number, _removeInfo: browser.Tabs.OnRemovedRemoveInfoType): void {}

async function onTabActivated(_activeInfo: browser.Tabs.OnActivatedActiveInfoType): Promise<void> {
	if (!activeNativePort) return;
	await sendMetadataForActiveTab();
}

async function onTabUpdated(
	_tabId: number,
	changeInfo: browser.Tabs.OnUpdatedChangeInfoType,
	tab: browser.Tabs.Tab,
): Promise<void> {
	if (changeInfo.status !== 'complete') return;
	if (!activeNativePort) return;
	if (!tab.active) return;
	await sendMetadataForActiveTab();
}

async function onWindowFocusChanged(windowId: number): Promise<void> {
	if (windowId === browser.windows.WINDOW_ID_NONE) return;
	if (!activeNativePort) return;
	await sendMetadataForActiveTab();
}

function isRealWebUrl(url: string): boolean {
	if (!url || typeof url !== 'string') return false;
	return /^https?:\/\//i.test(url);
}

async function sendMetadataForActiveTab(): Promise<void> {
	if (!activeNativePort) return;
	try {
		const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
		if (!activeTab || !activeTab.id || !activeTab.url || !isRealWebUrl(activeTab.url)) return;
		await sendMetadataEvent(activeTab, activeNativePort);
	} catch (error) {
		console.error('sendMetadataForActiveTab failed:', error);
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
				title: tab.title ?? null,
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
	} catch (error) {
		console.error('Failed to send metadata event:', error);
	}
}
