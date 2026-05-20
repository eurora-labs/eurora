import { startContextObserver, stopContextObserver } from './context-observer';
import { resolveFaviconBase64 } from './favicon';
import { initFocusTracker, destroyFocusTracker } from './focus-tracker';
import { startSafariPoller, stopSafariPoller } from './safari-poller';
import { errorFrame, forwardTabRpc } from './tab-rpc';
import { type TabStateBus, startTabStateBus } from './tab-state-bus';
import { isSafari } from './util';
import browser from 'webextension-polyfill';
import type { Frame, Payload, RequestFrame, ResponseFrame } from '../content/bindings';

declare const __DEV__: boolean;
const host = __DEV__ ? 'com.eurora.dev' : 'com.eurora.app';
const connectTimeout = 5000;
let nativePort: browser.Runtime.Port | null = null;
let tabBus: TabStateBus | null = null;

export function startNativeMessenger() {
	connect();
}

function connect() {
	nativePort = browser.runtime.connectNative(host);
	nativePort.onDisconnect.addListener(onNativePortDisconnect);
	nativePort.onMessage.addListener(onNativePortMessage);
	// One bus per connection lifecycle. Both observers subscribe; the
	// bus owns the underlying `chrome.tabs` / `chrome.windows` listeners.
	tabBus = startTabStateBus();
	initFocusTracker(nativePort, tabBus);
	startContextObserver(nativePort, tabBus);
	startSafariPoller();
}

function onNativePortDisconnect(port: browser.Runtime.Port) {
	const error = port.error;
	console.error('Native port disconnected:', error || 'Unknown error');

	destroyFocusTracker();
	stopContextObserver();
	tabBus?.stop();
	tabBus = null;
	stopSafariPoller();
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
		const resp = kind.Response as { action?: string };
		const pollerActions = ['POLL_REQUESTS', 'GET_METADATA'];
		if (!pollerActions.includes(resp.action ?? '')) {
			console.warn('Unexpected response frame:', kind.Response);
		}
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

		case 'YOUTUBE_GET_CURRENT_TIMESTAMP':
			return await forwardTabRpc(frame, 'GET_CURRENT_TIMESTAMP');

		case 'YOUTUBE_GET_TRANSCRIPT':
			return await forwardTabRpc(frame, 'GET_TRANSCRIPT');

		case 'YOUTUBE_GET_CURRENT_FRAME':
			return await forwardTabRpc(frame, 'GET_CURRENT_FRAME');

		case 'WEB_GET_PAGE_METADATA':
			return await forwardTabRpc(frame, 'GET_PAGE_METADATA');

		case 'WEB_GET_ACCESSIBILITY_TREE':
			return await forwardTabRpc(frame, 'GET_ACCESSIBILITY_TREE');

		case 'WEB_GET_READABILITY_ARTICLE':
			return await forwardTabRpc(frame, 'GET_READABILITY_ARTICLE');

		case 'WEB_GET_SELECTED_TEXT':
			return await forwardTabRpc(frame, 'GET_SELECTED_TEXT');

		case 'WEB_QUERY_SELECTOR':
			return await forwardTabRpc(frame, 'QUERY_SELECTOR');

		case 'WEB_LIST_LINKS':
			return await forwardTabRpc(frame, 'LIST_LINKS');

		case 'WEB_LIST_FORM_INPUTS':
			return await forwardTabRpc(frame, 'LIST_FORM_INPUTS');

		case 'WEB_INSERT_TEXT':
			return await forwardTabRpc(frame, 'INSERT_TEXT');

		default:
			return errorFrame(frame, 400, `Unknown action: ${frame.action}`);
	}
}

async function onActionMetadata(frame: RequestFrame): Promise<Frame> {
	const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
	const iconBase64 = await resolveFaviconBase64(activeTab);

	const response: ResponseFrame = {
		id: frame.id,
		action: frame.action,
		payload: {
			kind: 'NativeMetadata',
			data: {
				url: activeTab?.url,
				icon_base64: iconBase64,
				title: activeTab?.title ?? null,
			},
		} as Payload,
	};

	return { kind: { Response: response } } as Frame;
}

async function onActionMetadataFromContentScript(frame: RequestFrame): Promise<Frame> {
	const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });

	if (!activeTab || !activeTab.id) {
		const response: ResponseFrame = {
			id: frame.id,
			action: frame.action,
			payload: {
				kind: 'Error',
				data: 'No active tab found',
			} as Payload,
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
			payload: contentResponse as Payload,
		};

		return { kind: { Response: response } } as Frame;
	} catch (error) {
		const errorMessage = error instanceof Error ? error.message : String(error);
		const response: ResponseFrame = {
			id: frame.id,
			action: frame.action,
			payload: {
				kind: 'Error',
				data: `Failed to get metadata from content script: ${errorMessage}`,
			} as Payload,
		};
		return { kind: { Response: response } } as Frame;
	}
}
