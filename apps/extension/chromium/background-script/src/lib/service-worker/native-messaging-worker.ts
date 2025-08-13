// Native Messaging Service Worker - centralized handler for all native messaging
// Keep track of the native port connection
let nativePort: chrome.runtime.Port | null = null;

// Store queued messages if connection isn't ready
const messageQueue: any[] = [];

async function connect() {
	nativePort = chrome.runtime.connectNative('com.eurora.app');
	nativePort.onMessage.addListener(onMessageListener);
	nativePort.onDisconnect.addListener(onDisconnectListener);
}

async function onMessageListener(message: any, sender: chrome.runtime.Port) {
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
