import { onUpdated, onActivated } from './focus-tracker.js';
import { handleMessage } from './messaging.js';
import { getCurrentTabIcon } from './tabs.js';
import { isSafari } from './util.js';
import browser from 'webextension-polyfill';
import type { Frame, RequestFrame, ResponseFrame } from '../content/bindings.js';

// Native messaging host identifier
// For Chrome/Firefox: matches native-messaging-host.json ('com.eurora.app')
// For Safari: must match the containing app's bundle identifier ('com.eurora.macos')
function getHost(): string {
	// return isSafari() ? 'com.eurora.macos' : 'com.eurora.app';
	return 'com.eurora.app';
}
const connectTimeout = 5000;
let nativePort: browser.Runtime.Port | null = null;

export function startNativeMessenger() {
	const host = getHost();
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
}

function onNativePortDisconnect(port: browser.Runtime.Port) {
	const error = port.error;
	console.error('Native port disconnected:', error || 'Unknown error');
	nativePort = null;

	// Try to reconnect after a delay
	setTimeout(() => {
		connect();
	}, connectTimeout);
}

function connect() {
	const host = getHost();
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
			console.log('GET_METADATA');
			if (isSafari()) {
				return await onActionMetadataFromContentScript(frame);
			}
			return await onActionMetadata(frame);
		default: {
			const response = await handleMessage(frame.action);
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
}

async function onActionMetadata(frame: RequestFrame): Promise<Frame> {
	const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
	const iconBase64 = await getCurrentTabIcon(activeTab);

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

async function onActionMetadataFromContentScript(frame: RequestFrame): Promise<Frame> {
	const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });

	if (!activeTab.id) {
		const response: ResponseFrame = {
			id: frame.id,
			action: frame.action,
			payload: JSON.stringify({
				kind: 'Error',
				data: 'No active tab found',
			}),
		};
		return {
			kind: {
				Response: response,
			},
		} as Frame;
	}

	try {
		const contentResponse = await browser.tabs.sendMessage(activeTab.id, {
			type: 'GET_METADATA',
		});

		const response: ResponseFrame = {
			id: frame.id,
			action: frame.action,
			payload: JSON.stringify(contentResponse),
		};

		return {
			kind: {
				Response: response,
			},
		} as Frame;
	} catch (error) {
		const errorMessage = error instanceof Error ? error.message : String(error);
		const response: ResponseFrame = {
			id: frame.id,
			action: frame.action,
			payload: JSON.stringify({
				kind: 'Error',
				data: `Failed to get metadata from content script: ${errorMessage}`,
			}),
		};
		return {
			kind: {
				Response: response,
			},
		} as Frame;
	}
}
