import { getCurrentTabIcon } from './tabs.js';
import browser from 'webextension-polyfill';
import { NativeMetadata } from '../content/bindings.js';

const lastUrl = new Map();

// Frame protocol types matching the proto definition
interface Payload {
	kind: string;
	content: string; // JSON-encoded string
}

interface Frame {
	kind: string;
	id: number;
	action: string;
	event: string;
	payload?: Payload;
	ok: boolean;
}

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

			const metadata = {
				kind: 'NativeMetadata',
				data: {
					url: changeInfo.url,
					icon_base64: await getCurrentTabIcon(tab),
				} as NativeMetadata,
			};

			const frame: Frame = {
				kind: 'event',
				id: 0,
				action: '',
				event: 'tab_updated',
				payload: {
					kind: 'NativeMetadata',
					content: JSON.stringify(metadata),
				},
				ok: true,
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
		console.log(`[Tab Activated] ${url}`);

		const metadata = {
			kind: 'NativeMetadata',
			data: {
				url,
				icon_base64: await getCurrentTabIcon(tab),
			} as NativeMetadata,
		};

		const frame: Frame = {
			kind: 'event',
			id: 0,
			action: '',
			event: 'tab_activated',
			payload: {
				kind: 'NativeMetadata',
				content: JSON.stringify(metadata),
			},
			ok: true,
		};

		nativePort.postMessage(frame);
	} catch (error) {
		console.error(error);
	}
}

export async function onRemoved(tabId: number) {
	lastUrl.delete(tabId);
}
