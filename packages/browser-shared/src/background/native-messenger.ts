import { initFocusTracker, destroyFocusTracker } from './focus-tracker.js';
import { handleMessage } from './messaging.js';
import { getCurrentTabIcon } from './tabs.js';
import { isSafari } from './util.js';
import browser from 'webextension-polyfill';
import type { Frame, RequestFrame, ResponseFrame } from '../content/bindings.js';

const host = 'com.eurora.app';
const connectTimeout = 5000;
let nativePort: browser.Runtime.Port | null = null;

export function startNativeMessenger() {
	connect();
}

function connect() {
	nativePort = browser.runtime.connectNative(host);
	nativePort.onDisconnect.addListener(onNativePortDisconnect);
	nativePort.onMessage.addListener(onNativePortMessage);

	// Hand the port to the focus tracker so it can push events proactively.
	initFocusTracker(nativePort);
}

function onNativePortDisconnect(port: browser.Runtime.Port) {
	const error = port.error;
	console.error('Native port disconnected:', error || 'Unknown error');

	// Tear down the focus tracker before we lose the port reference.
	destroyFocusTracker();
	nativePort = null;

	// Try to reconnect after a delay.
	setTimeout(() => {
		connect();
	}, connectTimeout);
}

/**
 * Handle inbound messages from the native host.
 *
 * With the push-based model the extension proactively sends metadata, assets
 * and snapshots to the app.  The app should no longer need to send Request
 * frames for GENERATE_ASSETS / GENERATE_SNAPSHOT / GET_METADATA, but we keep
 * a lightweight request handler as a fallback so the protocol remains
 * backwards-compatible.
 */
async function onNativePortMessage(message: unknown, sender: browser.Runtime.Port) {
	// -----------------------------------------------------------------------
	// Safari dispatch-message format
	// SFSafariApplication.dispatchMessage sends:
	//   { name: string, userInfo: { frame, frameJson, action, requestId } }
	// -----------------------------------------------------------------------
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

		// For Safari dispatch messages send the response via sendNativeMessage
		// so it travels through SafariWebExtensionHandler → NativeMessagingBridge
		// → LocalBridgeServer instead of through the (unreliable) dispatch path.
		if (isSafari() && safariMessage.name === 'NativeRequest') {
			try {
				await browser.runtime.sendNativeMessage('com.eurora.app', response);
			} catch (error) {
				console.error('Failed to send response via sendNativeMessage:', error);
				sender.postMessage(response);
			}
		} else {
			sender.postMessage(response);
		}
	} else if ('Response' in kind) {
		// We don't expect Response frames from the native side.
		console.warn('Unexpected response frame:', kind.Response);
	} else if ('Event' in kind) {
		// The app shouldn't send events to the extension in normal operation.
		console.warn('Received event frame from native host:', kind.Event);
	} else if ('Error' in kind) {
		// Log but don't crash – the Safari bridge may return timeout errors for
		// fire-and-forget Event frames.  This is harmless.
		console.warn('Received error frame from native host:', kind.Error);
	} else if ('Cancel' in kind) {
		console.warn('Received cancel frame from native host:', kind.Cancel);
	}

	return true;
}

// ---------------------------------------------------------------------------
// Request handling (fallback – the app normally does NOT send these any more)
// ---------------------------------------------------------------------------

async function onRequestFrame(frame: RequestFrame): Promise<Frame> {
	switch (frame.action) {
		case 'GET_METADATA':
			if (isSafari()) {
				return await onActionMetadataFromContentScript(frame);
			}
			return await onActionMetadata(frame);

		case 'GENERATE_ASSETS':
		case 'GENERATE_SNAPSHOT': {
			const response = await handleMessage(frame.action);
			const responseFrame: ResponseFrame = {
				id: frame.id,
				action: frame.action,
				payload: JSON.stringify(response),
			};
			return { kind: { Response: responseFrame } } as Frame;
		}

		default: {
			const response = await handleMessage(frame.action);
			const responseFrame: ResponseFrame = {
				id: frame.id,
				action: frame.action,
				payload: JSON.stringify(response),
			};
			return { kind: { Response: responseFrame } } as Frame;
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
