import { getCurrentTabIcon } from './tabs.js';
import browser from 'webextension-polyfill';
import { NativeMetadata } from '../content/bindings.js';

const lastUrl = new Map();

function isRealWebUrl(url: string): boolean {
	if (!url || typeof url !== 'string') return false;
	if (!/^https?:\/\//i.test(url)) return false;
	return true;
}

export async function onUpdated(
	tabId: number,
	changeInfo: browser.Tabs.OnUpdatedChangeInfoType,
	tab: browser.Tabs.Tab,
	nativePort: browser.Runtime.Port,
): Promise<void> {
	if (typeof changeInfo.url === 'string' && isRealWebUrl(changeInfo.url)) {
		const prev = lastUrl.get(tabId);
		if (prev !== changeInfo.url) {
			console.log(`[URL Changed] ${changeInfo.url}`);
			lastUrl.set(tabId, changeInfo.url);

			nativePort.postMessage({
				kind: 'NativeMetadata',
				data: {
					url: changeInfo.url,
					icon_base64: await getCurrentTabIcon(tab),
				} as NativeMetadata,
			});
		}
	}
}

export async function onActivated(tabId: number, nativePort: browser.Runtime.Port): Promise<void> {
	try {
		const tab = await browser.tabs.get(tabId);
		if (!tab) return;
		const url = tab.url;
		if (!url || !isRealWebUrl(url)) return;
		console.log(`[Tab Activated] ${url}`);
		nativePort.postMessage({
			kind: 'NativeMetadata',
			data: {
				url,
				icon_base64: await getCurrentTabIcon(tab),
			} as NativeMetadata,
		});
	} catch (error) {
		console.error(error);
	}
}

export async function onRemoved(tabId: number) {
	lastUrl.delete(tabId);
}
