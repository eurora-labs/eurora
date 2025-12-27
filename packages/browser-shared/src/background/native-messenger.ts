import { onUpdated, onActivated } from './focus-tracker.js';
import { handleMessage } from './messaging.js';
import { getCurrentTabIcon } from './tabs.js';
import browser from 'webextension-polyfill';
import type { Frame, RequestFrame, ResponseFrame } from '../content/bindings.js';

const host = 'com.eurora.app';
const connectTimeout = 5000;
let nativePort: browser.Runtime.Port | null = null;

export function startNativeMessenger() {
	nativePort = browser.runtime.connectNative(host);
	nativePort.onDisconnect.addListener(onNativePortDisconnect);
	nativePort?.onMessage.addListener(onNativePortMessage);
	browser.tabs.onUpdated.addListener(async (tabId, changeInfo, tab) => {
		if (!nativePort) return;

		await onUpdated(tabId, changeInfo, tab, nativePort);
	});

	browser.tabs.onActivated.addListener(async (activeInfo) => {
		if (!nativePort) return;

		await onActivated(activeInfo.tabId, nativePort);
	});

	console.log('Native messaging service worker registered');
}

function onNativePortDisconnect(port: browser.Runtime.Port) {
	const error = port.error;
	console.error('Native port disconnected:', error?.message || 'Unknown error');
	nativePort = null;

	// Try to reconnect after a delay
	setTimeout(() => {
		connect();
		console.log('Reconnected to native host');
	}, connectTimeout);
}

function connect() {
	nativePort = browser.runtime.connectNative(host);
	nativePort.onDisconnect.addListener(onNativePortDisconnect);
	nativePort?.onMessage.addListener(onNativePortMessage);
}

async function onNativePortMessage(message: unknown, sender: browser.Runtime.Port) {
	// Assert type here
	const frame = message as Frame;
	const kind = frame.kind;
	if (!kind) {
		console.error('Invalid frame kind');
		throw new Error('Invalid frame kind');
	}

	console.log('kind: ', kind);
	if ('Request' in kind) {
		sender.postMessage(await onRequestFrame(kind.Request));
	} else if ('Response' in kind) {
		console.warn('Unexpected response frame: ', kind.Response);
	} else if ('Event' in kind) {
		console.warn('Event frames are not supported. Received: ', kind.Event);
	} else if ('Error' in kind) {
		console.error('Error frames are not supported. Received: ', kind.Error);
	} else if ('Cancel' in kind) {
		console.warn('Cancel frames are not supported. Received: ', kind.Cancel);
	}

	return true;
}

async function onRequestFrame(frame: RequestFrame): Promise<Frame> {
	switch (frame.action) {
		case 'GET_METADATA':
			return await onActionMetadata(frame);
		default:
			const response = await handleMessage(frame.action);
			console.log('Finished responding to ', frame.action, ': ', response);
			const responseFrame: ResponseFrame = {
				id: frame.id,
				action: frame.action,
				payload: JSON.stringify(response),
			};
			return {
				kind: {
					Response: responseFrame,
				},
			} as Frame;
	}
}

async function onResponseFrame(frame: ResponseFrame): Promise<Frame> {
	throw new Error('Not implemented');
}

async function onActionMetadata(frame: RequestFrame): Promise<Frame> {
	const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
	const iconBase64 = await getCurrentTabIcon(activeTab);
	console.log('Tab metadata:', { url: activeTab.url, icon_base64: iconBase64 });

	const response: ResponseFrame = {
		id: frame.id,
		action: frame.action,
		payload: JSON.stringify({
			kind: 'NativeMetadata',
			data: {
				url: activeTab.url,
				icon_base64: iconBase64,
			},
		}),
	};

	return {
		kind: {
			Response: response,
		},
	} as Frame;
}
