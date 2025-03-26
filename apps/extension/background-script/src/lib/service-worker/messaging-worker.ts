// This file is deprecated - all native messaging functionality is now in native-messaging-worker.ts
// Keep this file as a placeholder until references are updated
// Native Messaging Service Worker
let nativePort: chrome.runtime.Port | null = null;

// Handle content script messages - DEPRECATED, see native-messaging-worker.ts
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
	// Just forward to the new messaging worker
	return false;
});

// Native messaging connection management
function connectToNativeHost() {
	try {
		nativePort = chrome.runtime.connectNative('com.eurora.app.dev');

		nativePort.onMessage.addListener((response) => {
			// Broadcast responses to all content scripts
			chrome.tabs.query({}, (tabs) => {
				tabs.forEach((tab) => {
					chrome.tabs.sendMessage(tab.id!, {
						type: 'NATIVE_RESPONSE',
						payload: response
					});
				});
			});
		});

		nativePort.onDisconnect.addListener(() => {
			console.log('Native port disconnected');
			nativePort = null;
			setTimeout(connectToNativeHost, 5000);
		});
	} catch (error) {
		console.error('Native connection failed:', error);
		setTimeout(connectToNativeHost, 5000);
	}
}

function forwardToNativeHost(payload: any) {
	if (!nativePort) {
		console.error('No active native connection');
		return;
	}

	try {
		nativePort.postMessage({
			type: 'TRANSCRIPT',
			videoId: payload.videoId,
			transcript: JSON.stringify(payload.transcript)
		});
	} catch (error) {
		console.error('Message sending failed:', error);
	}
}

// Initial connection
connectToNativeHost();
