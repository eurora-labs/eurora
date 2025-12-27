import { getCurrentTabIcon } from './tabs.js';
import { NativeMetadata, Frame } from '../content/bindings.js';
import browser from 'webextension-polyfill';

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
	if (changeInfo.status !== 'complete') return;

	if (typeof tab.url === 'string' && isRealWebUrl(tab.url)) {
		const prev = lastUrl.get(tabId);
		if (prev !== tab.url) {
			lastUrl.set(tabId, tab.url);

			const metadata = {
				kind: 'NativeMetadata',
				data: {
					url: tab.url,
					icon_base64: await getCurrentTabIcon(tab),
				} as NativeMetadata,
			};

			const frame: Frame = {
				kind: {
					Event: {
						action: 'TAB_UPDATED',
						payload: JSON.stringify(metadata),
					},
				},
			};

			nativePort.postMessage(frame);
		}
	}
}

export async function onActivated(tabId: number, nativePort: browser.Runtime.Port): Promise<void> {
	try {
		const tab = await browser.tabs.get(tabId);
		if (!tab) return;
		const url = tab.url;
		if (!url || !isRealWebUrl(url)) return;

		const metadata = {
			kind: 'NativeMetadata',
			data: {
				url,
				icon_base64: await getCurrentTabIcon(tab),
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

		nativePort.postMessage(frame);
	} catch (error) {
		console.error(error);
	}
}

export async function onRemoved(tabId: number) {
	lastUrl.delete(tabId);
}
