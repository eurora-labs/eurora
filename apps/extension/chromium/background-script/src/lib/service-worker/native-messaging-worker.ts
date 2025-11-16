// Native Messaging Service Worker - centralized handler for all native messaging
// Keep track of the native port connection
import { handleMessage } from '@eurora/browser-shared/background/messaging';
import { getCurrentTabIcon } from '@eurora/browser-shared/background/tabs';
import { onUpdated, onActivated } from '@eurora/browser-shared/background/focus-tracker';
import { Frame } from '@eurora/browser-shared/content/bindings';

let nativePort: chrome.runtime.Port | null = null;

chrome.tabs.onUpdated.addListener(async (tabId, changeInfo, tab) => {
	if (!nativePort) return;

	await onUpdated(tabId, changeInfo, tab, nativePort);
});

chrome.tabs.onActivated.addListener(async (activeInfo) => {
	if (!nativePort) return;

	await onActivated(activeInfo.tabId, nativePort);
});

async function connect() {
	console.log('Connecting to native messaging app');
	nativePort = chrome.runtime.connectNative('com.eurora.app');
	nativePort.onMessage.addListener(onMessageListener);
	nativePort.onDisconnect.addListener(onDisconnectListener);
}

async function onMessageListener(frame: Frame, sender: chrome.runtime.Port) {
	console.log('Received frame:', frame);

	switch (frame.action) {
		case 'GET_METADATA':
			try {
				const [activeTab] = await chrome.tabs.query({ active: true, currentWindow: true });
				const iconBase64 = await getCurrentTabIcon(activeTab);
				console.log('Tab metadata:', { url: activeTab.url, icon_base64: iconBase64 });

				const responseData = {
					kind: 'NativeMetadata',
					data: {
						url: activeTab.url,
						icon_base64: iconBase64,
					},
				};

				const responseFrame: Frame = {
					kind: 'response',
					id: frame.id, // Echo back the request ID
					action: frame.action,
					event: '',
					payload: {
						kind: 'NativeMetadata',
						content: JSON.stringify(responseData),
					},
					ok: true,
				};

				sender.postMessage(responseFrame);
			} catch (error) {
				console.error('Error getting tab metadata:', error);
				const errorFrame: Frame = {
					kind: 'response',
					id: frame.id,
					action: frame.action,
					event: '',
					payload: undefined,
					ok: false,
				};
				sender.postMessage(errorFrame);
			}
			break;
		default:
			try {
				// Handle assets request using the existing handleMessage
				const response = await handleMessage(frame.action);
				console.log('Finished responding to ', frame.action, ': ', response);

				const responseFrame: Frame = {
					kind: 'response',
					id: frame.id,
					action: frame.action,
					event: '',
					payload: {
						kind: response.kind || 'unknown',
						content: JSON.stringify(response),
					},
					ok: true,
				};

				sender.postMessage(responseFrame);
			} catch (error) {
				console.error('Error responding to ', frame.action, ': ', error);
				const errorFrame: Frame = {
					kind: 'response',
					id: frame.id,
					action: frame.action,
					event: '',
					payload: undefined,
					ok: false,
				};
				sender.postMessage(errorFrame);
			}
			break;
	}
	return true;
}

function onDisconnectListener() {
	const error = chrome.runtime.lastError;
	console.error('Native port disconnected:', error?.message || 'Unknown error');
	nativePort = null;

	// Try to reconnect after a delay
	setTimeout(() => {
		connect().then(() => {
			console.log('Reconnected to native host');
		});
	}, 5000);
}

connect();

console.log('Native messaging service worker registered');
