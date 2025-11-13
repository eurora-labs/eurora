// Native Messaging Service Worker - centralized handler for all native messaging
// Keep track of the native port connection
import { handleMessage } from '@eurora/browser-shared/background/messaging';
import { getCurrentTabIcon } from '@eurora/browser-shared/background/tabs';
import { onUpdated, onActivated } from '@eurora/browser-shared/background/focus-tracker';

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

async function onMessageListener(message: { command: string }, sender: chrome.runtime.Port) {
	switch (message.command) {
		case 'GET_METADATA':
			try {
				const [activeTab] = await chrome.tabs.query({ active: true, currentWindow: true });
				const iconBase64 = await getCurrentTabIcon(activeTab);
				sender.postMessage({
					kind: 'NativeMetadata',
					data: {
						url: activeTab.url,
						icon_base64: iconBase64,
					},
				});
			} catch (error) {
				console.error('Error getting tab metadata:', error);
				sender.postMessage({
					kind: 'NativeMetadata',
					data: {
						url: undefined,
						icon_base64: undefined,
					},
				});
			}
			break;
		case 'GET_ICON':
			try {
				const [activeTab] = await chrome.tabs.query({ active: true, currentWindow: true });
				const iconBase64 = await getCurrentTabIcon(activeTab);
				sender.postMessage({
					kind: 'NativeIcon',
					data: {
						base64: iconBase64,
					},
				});
			} catch (error) {
				console.error('Error getting tab icon:', error);
				sender.postMessage({
					kind: 'NativeIcon',
					data: {
						base64: undefined,
					},
				});
			}
			break;
		default:
			handleMessage(message.command)
				.then((response) => {
					console.log('Finished responding to type: ', message.command);
					sender.postMessage(response);
				})
				.catch((error) => {
					console.error('Error responding to message', error);
					sender.postMessage({ success: false, error: String(error) });
				});
			break;
	}
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
