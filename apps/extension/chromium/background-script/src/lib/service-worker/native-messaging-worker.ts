// Native Messaging Service Worker - centralized handler for all native messaging
// Keep track of the native port connection
import { handleMessage } from '@eurora/browser-shared/background/messaging';
import { getCurrentTabIcon } from '@eurora/browser-shared/background/tabs';
import { onUpdated, onActivated } from '@eurora/browser-shared/background/focus-tracker';
import { Frame, type Id } from '@eurora/browser-shared/content/bindings';

let nativePort: chrome.runtime.Port | null = null;

function addBase64Prefix(base64: string) {
	const head = base64.substring(0, 6);
	switch (head) {
		case 'PHN2Zy':
			return `data:image/svg+xml;base64,${base64}`;
		case 'CiAgPH':
			return `data:image/svg+xml;base64,${base64.substring(4)}`;

		default:
			return base64;
	}
}

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

	let frameId = 0;
	// For now this is fine as Chrome doesn't send messages expecting a response
	if ('Request' in frame.id) {
		frameId = frame.id.Request;
	} else {
		throw new Error('Invalid frame ID: ' + frame.id);
	}

	switch (frame.command) {
		case 'GET_METADATA':
			try {
				const [activeTab] = await chrome.tabs.query({ active: true, currentWindow: true });
				const iconBase64 = addBase64Prefix(await getCurrentTabIcon(activeTab));
				console.log('Tab metadata:', { url: activeTab.url, icon_base64: iconBase64 });

				const responseData = {
					kind: 'NativeMetadata',
					data: {
						url: activeTab.url,
						icon_base64: iconBase64,
					},
				};

				const responseFrame: Frame = {
					id: { Response: frameId },
					command: frame.command,
					payload: JSON.stringify(responseData),
				};

				sender.postMessage(responseFrame);
			} catch (error) {
				console.error('Error getting tab metadata:', error);
				const errorFrame: Frame = {
					id: { Error: frameId },
					command: frame.command,
					payload: undefined,
				};
				sender.postMessage(errorFrame);
			}
			break;
		default:
			try {
				// Handle assets request using the existing handleMessage
				const response = await handleMessage(frame.command);
				console.log('Finished responding to ', frame.command, ': ', response);

				const responseFrame: Frame = {
					id: { Response: frameId },
					command: frame.command,
					payload: JSON.stringify(response),
				};

				sender.postMessage(responseFrame);
			} catch (error) {
				console.error('Error responding to ', frame.command, ': ', error);
				const errorFrame: Frame = {
					id: { Error: frameId },
					command: frame.command,
					payload: undefined,
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
