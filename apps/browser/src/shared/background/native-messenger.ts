import { initFocusTracker, destroyFocusTracker } from './focus-tracker';
import { getCurrentTabIcon } from './tabs';
import { isSafari } from './util';
import browser from 'webextension-polyfill';
import type { Frame, RequestFrame, ResponseFrame } from '../content/bindings';

declare const __DEV__: boolean;
const host = __DEV__ ? 'com.eurora.dev' : 'com.eurora.app';
const connectTimeout = 5000;
let nativePort: browser.Runtime.Port | null = null;

export function startNativeMessenger() {
	connect();
}

function connect() {
	nativePort = browser.runtime.connectNative(host);
	nativePort.onDisconnect.addListener(onNativePortDisconnect);
	nativePort.onMessage.addListener(onNativePortMessage);
	initFocusTracker(nativePort);
}

function onNativePortDisconnect(port: browser.Runtime.Port) {
	const error = port.error;
	console.error('Native port disconnected:', error || 'Unknown error');

	destroyFocusTracker();
	nativePort = null;

	setTimeout(() => {
		connect();
	}, connectTimeout);
}

async function onNativePortMessage(message: unknown, sender: browser.Runtime.Port) {
	const safariMessage = message as {
		name?: string;
		userInfo?: { frame?: Frame; frameJson?: string; action?: string; requestId?: string };
	};

	let frame: Frame;

	if (safariMessage.name === 'NativeRequest' && safariMessage.userInfo?.frame) {
		frame = safariMessage.userInfo.frame;
	} else {
		frame = message as Frame;
	}

	const kind = frame.kind;
	if (!kind) {
		console.error('Invalid frame kind');
		return;
	}

	if ('Request' in kind) {
		const response = await onRequestFrame(kind.Request);

		if (isSafari() && safariMessage.name === 'NativeRequest') {
			try {
				await browser.runtime.sendNativeMessage(host, response);
			} catch (error) {
				console.error('Failed to send response via sendNativeMessage:', error);
				sender.postMessage(response);
			}
		} else {
			sender.postMessage(response);
		}
	} else if ('Response' in kind) {
		console.warn('Unexpected response frame:', kind.Response);
	} else if ('Event' in kind) {
		console.warn('Received event frame from native host:', kind.Event);
	} else if ('Error' in kind) {
		console.warn('Received error frame from native host:', kind.Error);
	} else if ('Cancel' in kind) {
		console.warn('Received cancel frame from native host:', kind.Cancel);
	}

	return true;
}

async function onRequestFrame(frame: RequestFrame): Promise<Frame> {
	switch (frame.action) {
		case 'GET_METADATA':
			if (isSafari()) {
				return await onActionMetadataFromContentScript(frame);
			}
			return await onActionMetadata(frame);

		default:
			return {
				kind: {
					Error: {
						id: frame.id,
						code: 0,
						message: `Unknown action: ${frame.action}`,
						details: null,
					},
				},
			} as Frame;
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
				url: activeTab?.url,
				icon_base64: iconBase64,
			},
		}),
	};

	return { kind: { Response: response } } as Frame;
}

async function onActionMetadataFromContentScript(frame: RequestFrame): Promise<Frame> {
	const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });

	if (!activeTab || !activeTab.id) {
		const response: ResponseFrame = {
			id: frame.id,
			action: frame.action,
			payload: JSON.stringify({
				kind: 'Error',
				data: 'No active tab found',
			}),
		};
		return { kind: { Response: response } } as Frame;
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

		return { kind: { Response: response } } as Frame;
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
		return { kind: { Response: response } } as Frame;
	}
}
