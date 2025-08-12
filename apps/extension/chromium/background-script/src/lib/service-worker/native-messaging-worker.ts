// Native Messaging Service Worker - centralized handler for all native messaging
// Keep track of the native port connection
let nativePort: chrome.runtime.Port | null = null;

handlePortDisconnect();

function handlePortDisconnect(disconnected = false) {
	if (disconnected) {
		setTimeout(() => {
			handlePortDisconnect();
		}, 5000);
		return;
	}
	connectToNativeHost().then(
		(connected) => {
			if (connected) processQueue();

			nativePort.onDisconnect.addListener(() => {
				handlePortDisconnect(true);
			});
		},
		(error) => {
			console.error('Failed to connect to native host:', error);
			nativePort = null;
			handlePortDisconnect(true);
		},
	);
}

// Store queued messages if connection isn't ready
const messageQueue: any[] = [];

// Initialize connection to native host
function connectToNativeHost(): Promise<boolean> {
	return new Promise((resolve) => {
		try {
			console.log('Connecting to native host...');
			nativePort = chrome.runtime.connectNative('com.eurora.app');

			nativePort.onMessage.addListener((response) => {
				console.log('Received response from native host:', response);

				// Broadcast response to all tabs
				chrome.tabs.query({}, (tabs) => {
					tabs.forEach((tab) => {
						console.log('Sending message to tab', tab.id);
						console.log('Response', response);
						if (tab.id) {
							chrome.tabs.sendMessage(tab.id, {
								type: 'NATIVE_RESPONSE',
								payload: response,
							});
						}
					});
				});
			});

			nativePort.onDisconnect.addListener(() => {
				const error = chrome.runtime.lastError;
				console.error('Native port disconnected:', error?.message || 'Unknown error');
				nativePort = null;

				// Try to reconnect after a delay
				setTimeout(() => {
					connectToNativeHost().then((connected) => {
						if (connected) processQueue();
					});
				}, 5000);
			});

			console.log('Successfully connected to native host');
			resolve(true);
		} catch (error) {
			console.error('Failed to connect to native host:', error);
			nativePort = null;
			resolve(false);

			// Try to reconnect after a delay
			setTimeout(() => {
				connectToNativeHost().then((connected) => {
					if (connected) processQueue();
				});
			}, 5000);
		}
	});
}

// Process any queued messages
function processQueue() {
	console.log(`Processing queue with ${messageQueue.length} messages`);
	while (messageQueue.length > 0) {
		const { payload, tabId } = messageQueue.shift();
		sendMessageToNativeHost(payload, tabId);
	}
}

function sendMessageToNativeHost(payload: any, tabId?: number) {
	console.log('Sending message to native host:', payload);
	console.log('Native port:', nativePort);
	try {
		// Forward the payload directly as it should already be in protocol format
		// The payload comes from content scripts that construct proper protocol messages
		nativePort!.postMessage(payload);

		return { status: 'sent' };
	} catch (error) {
		console.error('Failed to send message to native host:', error);

		// Return error to caller
		const errorResponse = {
			status: 'error',
			error: error instanceof Error ? error.message : String(error),
		};

		// Notify content script of failure
		if (tabId) {
			chrome.tabs.sendMessage(tabId, {
				type: 'NATIVE_RESPONSE',
				payload: errorResponse,
			});
		}

		// Reconnect on error
		nativePort = null;
		setTimeout(() => {
			connectToNativeHost().then((connected) => {
				if (connected) processQueue();
			});
		}, 1000);

		return errorResponse;
	}
}

// // Initialize the connection when the service worker starts
connectToNativeHost().then((connected) => {
	console.log(
		`Native messaging service worker initialized, connection status: ${connected ? 'connected' : 'failed'}`,
	);
	nativePort.onMessage.addListener(async (message, sender) => {
		switch (message.type) {
			case 'GENERATE_ASSETS':
				handleGenerateReport()
					.then((response) => {
						console.log('Sending GENERATE_REPORT_RESPONSE message', response);
						sender.postMessage(response);
					})
					.catch((error) => {
						console.log('Error generating report', error);
						sender.postMessage({ success: false, error: String(error) });
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
				throw new Error(`Unknown message type: ${message.type}`);
		}
	});
});

console.log('Native messaging service worker registered');

import { getCurrentTab } from '../utils/tabs.ts';

async function handleGenerateSnapshot() {
	try {
		// Get the current active tab
		const activeTab = await getCurrentTab();

		if (!activeTab || !activeTab.url) {
			return { success: false, error: 'No active tab found' };
		}

		type Response = {
			error?: string;
			[key: string]: any;
		};

		const response: Response = await new Promise((resolve, reject) =>
			chrome.tabs.sendMessage(activeTab.id, { type: 'GENERATE_SNAPSHOT' }, (response) => {
				if (chrome.runtime.lastError) {
					reject({ error: chrome.runtime.lastError });
				} else if (response?.error) {
					reject({ error: response.error });
				} else {
					resolve(response);
				}
			}),
		);

		return { success: true, ...response };
	} catch (error) {
		console.error('Error generating snapshot:', error);
		return {
			success: false,
			error: String(error),
		};
	}
}

/**
 * Handles the GENERATE_REPORT message by getting the current active tab,
 * checking if it's a YouTube video or article page, and requesting a report
 * from the appropriate watcher
 */
async function handleGenerateReport() {
	try {
		// Get the current active tab
		const activeTab = await getCurrentTab();

		if (!activeTab || !activeTab.url) {
			return { success: false, error: 'No active tab found' };
		}

		type Response = {
			error?: string;
			[key: string]: any;
		};

		const response: Response = await new Promise((resolve, reject) =>
			chrome.tabs.sendMessage(activeTab.id, { type: 'GENERATE_ASSETS' }, (response) => {
				if (chrome.runtime.lastError) {
					reject({ error: chrome.runtime.lastError });
				} else if (response?.error) {
					reject({ error: response.error });
				} else {
					resolve(response);
				}
			}),
		);

		return { success: true, ...response };
	} catch (error) {
		console.error('Error generating report:', error);
		return {
			success: false,
			error: String(error),
		};
	}
}
