// Native Messaging Service Worker - centralized handler for all native messaging
// Keep track of the native port connection
import { handleGenerateAssets, handleGenerateSnapshot } from '@eurora/browser-shared/messaging';

let nativePort: chrome.runtime.Port | null = null;

// Store queued messages if connection isn't ready
const messageQueue: any[] = [];

async function connect() {
	console.log('Connecting to native messaging app');
	nativePort = chrome.runtime.connectNative('com.eurora.app');
	nativePort.onMessage.addListener(onMessageListener);
	nativePort.onDisconnect.addListener(onDisconnectListener);
}

async function onMessageListener(message: { command: string }, sender: chrome.runtime.Port) {
	switch (message.command) {
		case 'GENERATE_ASSETS':
			handleGenerateAssets()
				.then((response) => {
					console.log('Sending GENERATE_REPORT_RESPONSE message', response);
					sender.postMessage(response);
				})
				.catch((error) => {
					console.log('Error generating report', error);
					sender.postMessage(error);
				});
			return true; // Indicates we'll call sendResponse asynchronously
		case 'GENERATE_SNAPSHOT':
			handleGenerateSnapshot()
				.then((response) => {
					console.log('Sending GENERATE_SNAPSHOT_RESPONSE message', response);
					sender.postMessage(response);
				})
				.catch((error) => {
					console.log('Error generating snapshot', error);
					sender.postMessage({ success: false, error: String(error) });
				});
			return true; // Indicates we'll call sendResponse asynchronously
		default:
			console.log('Unknown message type:', message);
			sender.postMessage({ success: false, error: 'Unknown message type' });
			return false;
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
