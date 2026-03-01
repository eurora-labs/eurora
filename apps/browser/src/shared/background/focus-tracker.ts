import { sendMessageWithRetry } from './messaging';
import { getCurrentTabIcon } from './tabs';
import browser from 'webextension-polyfill';
import type { NativeMetadata, Frame } from '../content/bindings';

let activeNativePort: browser.Runtime.Port | null = null;
let collectGeneration = 0;

export function initFocusTracker(port: browser.Runtime.Port): void {
	activeNativePort = port;

	browser.tabs.onActivated.addListener(onTabActivated);
	browser.tabs.onUpdated.addListener(onTabUpdated);
	browser.tabs.onRemoved.addListener(onTabRemoved);
	browser.windows.onFocusChanged.addListener(onWindowFocusChanged);

	sendMetadataForActiveTab().catch(console.error);
	startCollecting().catch(console.error);
}

export function destroyFocusTracker(): void {
	collectGeneration++;
	activeNativePort = null;

	browser.tabs.onActivated.removeListener(onTabActivated);
	browser.tabs.onUpdated.removeListener(onTabUpdated);
	browser.tabs.onRemoved.removeListener(onTabRemoved);
	browser.windows.onFocusChanged.removeListener(onWindowFocusChanged);
}

export function setNativePort(port: browser.Runtime.Port | null): void {
	activeNativePort = port;
}

function onTabRemoved(_tabId: number, removeInfo: browser.Tabs.OnRemovedRemoveInfoType): void {
	if (removeInfo.isWindowClosing) {
		collectGeneration++;
	}
}

async function onTabActivated(_activeInfo: browser.Tabs.OnActivatedActiveInfoType): Promise<void> {
	if (!activeNativePort) return;
	collectGeneration++;
	await sendMetadataForActiveTab();
	startCollecting().catch(console.error);
}

async function onTabUpdated(
	_tabId: number,
	changeInfo: browser.Tabs.OnUpdatedChangeInfoType,
	tab: browser.Tabs.Tab,
): Promise<void> {
	if (changeInfo.status !== 'complete') return;
	if (!activeNativePort) return;
	if (!tab.active) return;
	collectGeneration++;
	await sendMetadataForActiveTab();
	startCollecting().catch(console.error);
}

async function onWindowFocusChanged(windowId: number): Promise<void> {
	if (windowId === browser.windows.WINDOW_ID_NONE) return;
	if (!activeNativePort) return;
	collectGeneration++;
	await sendMetadataForActiveTab();
	startCollecting().catch(console.error);
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

async function startCollecting(): Promise<void> {
	if (!activeNativePort) return;

	const gen = collectGeneration;

	const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
	if (gen !== collectGeneration) return;
	if (!activeTab || !activeTab.id || !activeTab.url || !isRealWebUrl(activeTab.url)) return;

	const tabId = activeTab.id;
	let firstRun = true;

	while (gen === collectGeneration && activeNativePort) {
		try {
			let assets: any = null;
			let snapshot: any = null;

			if (firstRun) {
				firstRun = false;
				const [a, s] = await Promise.all([
					sendMessageWithRetry(tabId, { type: 'GENERATE_ASSETS' }),
					sendMessageWithRetry(tabId, { type: 'GENERATE_SNAPSHOT' }),
				]);
				assets = a;
				snapshot = s;
			} else {
				const response = await sendMessageWithRetry(tabId, { type: 'COLLECT' });
				if (response && response.kind === 'CollectResponse' && response.data) {
					assets = response.data.assets;
					snapshot = response.data.snapshot;
				}
			}

			if (gen !== collectGeneration || !activeNativePort) break;

			const port = activeNativePort;

			if (assets && assets.kind !== 'Error') {
				const frame: Frame = {
					kind: {
						Event: {
							action: 'ASSETS',
							payload: JSON.stringify(assets),
						},
					},
				};
				port.postMessage(frame);
			}

			if (snapshot && snapshot.kind !== 'Error') {
				const frame: Frame = {
					kind: {
						Event: {
							action: 'SNAPSHOT',
							payload: JSON.stringify(snapshot),
						},
					},
				};
				port.postMessage(frame);
			}
		} catch (error) {
			if (gen !== collectGeneration) break;
			console.warn('COLLECT failed, retrying in 1s:', error);
			await new Promise((resolve) => setTimeout(resolve, 1000));
		}
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
	} catch (error) {
		console.error('Failed to send metadata event:', error);
	}
}
