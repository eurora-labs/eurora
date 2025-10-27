// Native Messaging Service Worker - centralized handler for all native messaging
// Keep track of the native port connection
import { handleMessage } from '@eurora/browser-shared/messaging';

let nativePort: chrome.runtime.Port | null = null;

async function connect() {
	console.log('Connecting to native messaging app');
	nativePort = chrome.runtime.connectNative('com.eurora.app');
	nativePort.onMessage.addListener(onMessageListener);
	nativePort.onDisconnect.addListener(onDisconnectListener);
}

async function onMessageListener(message: { command: string }, sender: chrome.runtime.Port) {
	handleMessage(message.command)
		.then((response) => {
			console.log('Finished responding to type: ', message.command);
			sender.postMessage(response);
		})
		.catch((error) => {
			console.error('Error responding to message', error);
			sender.postMessage({ success: false, error: String(error) });
		});
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
