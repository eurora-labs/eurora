// Native Messaging Service Worker - centralized handler for all native messaging
// Keep track of the native port connection
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
			handleGenerateReport()
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

import { getCurrentTab } from '../utils/tabs.ts';

async function handleGenerateSnapshot() {
	try {
		// Get the current active tab
		const activeTab = await getCurrentTab();

		if (!activeTab || !activeTab.url) {
			return { success: false, error: 'No active tab found' };
		}

		const response = await sendMessageWithRetry(activeTab.id, {
			type: 'GENERATE_SNAPSHOT',
		});

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
 * Sends a message to a tab with retry logic to handle content script initialization delays
 */
async function sendMessageWithRetry(
	tabId: number,
	message: any,
	maxRetries: number = 3,
	delayMs: number = 500,
): Promise<any> {
	for (let attempt = 0; attempt < maxRetries; attempt++) {
		try {
			const response = await chrome.tabs.sendMessage(tabId, message);
			return response;
		} catch (error) {
			const isLastAttempt = attempt === maxRetries - 1;
			const isConnectionError =
				error?.message?.includes('Receiving end does not exist') ||
				chrome.runtime.lastError?.message?.includes('Receiving end does not exist');

			if (isConnectionError && !isLastAttempt) {
				console.log(`Content script not ready, retrying (${attempt + 1}/${maxRetries})...`);
				await new Promise((resolve) => setTimeout(resolve, delayMs));
				continue;
			}
			throw error;
		}
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
			return { success: false, data: 'No active tab found', kind: 'Error' };
		}

		const response = await sendMessageWithRetry(activeTab.id, {
			type: 'GENERATE_ASSETS',
		});
		console.log('Async response:', response);

		return { success: true, ...response };
	} catch (error) {
		console.error('Error generating report:', error);
		return {
			kind: 'Error',
			success: false,
			data: String(error),
		};
	}
}
