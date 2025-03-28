// Native Messaging Service Worker - centralized handler for all native messaging

// Keep track of the native port connection
let nativePort: chrome.runtime.Port | null = null;

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
								payload: response
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

// Forward message to native host
function forwardToNativeHost(payload: any, tabId?: number) {
	if (!nativePort) {
		console.log('No active native connection, queuing message and connecting...');
		// Queue the message
		messageQueue.push({ payload, tabId });

		// Attempt to connect
		connectToNativeHost().then((connected) => {
			if (connected) processQueue();
		});

		// Return status to the caller
		return { status: 'queued', message: 'Native connection not available, message queued' };
	}

	// return sendMessageToNativeHost(payload, tabId);
}

function sendMessageToNativeHost(payload: any, tabId?: number) {
	console.log('Sending message to native host:', payload);
	console.log('Native port:', nativePort);
	try {
		// Format the message according to the native messaging protocol
		const nativeMessage = {
			type: 'TRANSCRIPT',
			videoId: payload.videoId || 'unknown',
			transcript:
				typeof payload.transcript === 'string'
					? payload.transcript
					: JSON.stringify(payload.transcript)
		};

		console.log('Sending message to native host:', nativeMessage);
		nativePort!.postMessage(nativeMessage);

		return { status: 'sent' };
	} catch (error) {
		console.error('Failed to send message to native host:', error);

		// Return error to caller
		const errorResponse = {
			status: 'error',
			error: error instanceof Error ? error.message : String(error)
		};

		// Notify content script of failure
		if (tabId) {
			chrome.tabs.sendMessage(tabId, {
				type: 'NATIVE_RESPONSE',
				payload: errorResponse
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
		`Native messaging service worker initialized, connection status: ${connected ? 'connected' : 'failed'}`
	);
	nativePort.onMessage.addListener(async (message, sender) => {
		// GENERATE_REPORT message
		if (message.type === 'GENERATE_REPORT') {
			console.log('Received GENERATE_REPORT message', message);
			// sender.postMessage({
			// 	type: 'GENERATE_REPORT_RESPONSE',
			// 	success: true,
			// 	url: 'URL FROM CHROME WORKER'
			// });
			// console.log('Sending GENERATE_REPORT_RESPONSE message');
			handleGenerateReport()
				.then((response) => {
					console.log('Sending GENERATE_REPORT_RESPONSE message', response);
					sender.postMessage(response);
				})
				.catch((error) => {
					console.log('Error generating report', error);
					sender.postMessage({ success: false, error: String(error) });
				});
			console.log('Sending true while waiting on report');
			return true; // Indicates we'll call sendResponse asynchronously
		}
	});
});

// // Handle service worker lifecycle
// chrome.runtime.onStartup.addListener(() => {
// 	console.log('Extension started, initializing native messaging');
// 	connectToNativeHost().then(() => {
// 		// Handle messages from content scripts
// 		nativePort.onMessage.addListener(async (message, sender) => {
// 			// GENERATE_REPORT message
// 			if (message.type === 'GENERATE_REPORT') {
// 				console.log('Received GENERATE_REPORT message');
// 				sender.postMessage({
// 					type: 'GENERATE_REPORT_RESPONSE',
// 					payload: {
// 						success: true,
// 						url: 'URL FROM CHROME WORKER'
// 					}
// 				});
// 				console.log('Sending GENERATE_REPORT_RESPONSE message');
// 				// handleGenerateReport()
// 				// 	.then((response) => sendResponse(response))
// 				// 	.catch((error) => sendResponse({ success: false, error: String(error) }));
// 				// console.log('Sending true while waiting on report');
// 				// return true; // Indicates we'll call sendResponse asynchronously
// 			}
// 		});
// 	});
// });

console.log('Native messaging service worker registered');

import { getCurrentTab } from '../utils/tabs.js';
import { isYouTubeVideoUrl, isPdfUrl } from '../utils/url-helpers.js';
import { isArticlePage } from '../utils/page-detection.js';

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

		console.log('Active tab', activeTab);

		// Check if current tab is a YouTube video
		if (isYouTubeVideoUrl(activeTab.url)) {
			// Request a report from YouTube Watcher
			console.log('Requesting YouTube report');
			let report;
			try {
				report = await requestYouTubeWatcherReport(activeTab.url);
			} catch (error) {
				console.error('Error requesting YouTube report:', error);
				return {
					success: false,
					error: String(error)
				};
			}
			console.log('YouTube report', report);
			return {
				type: 'YOUTUBE_STATE',
				success: true,
				...(report as any)
			};
		}

		// Check if current tab is a PDF document
		if (isPdfUrl(activeTab.url)) {
			// Request a report from PDF Watcher
			const report = await requestPdfWatcherReport(activeTab.url);
			return {
				type: 'PDF_STATE',
				success: true,
				...(report as any)
			};
		}

		const report = await requestArticleWatcherReport(activeTab.url);
		return {
			type: 'ARTICLE_STATE',
			success: true,
			...(report as any)
		};
		// else if (isArticlePage(activeTab.url)) {
		// 	// Request a report from Article Watcher

		// } else {
		// 	return {
		// 		success: false,
		// 		error: 'Current tab does not have a handler',
		// 		url: activeTab.url
		// 	};
		// }
	} catch (error) {
		console.error('Error generating report:', error);
		return {
			success: false,
			error: String(error)
		};
	}
}

async function requestPdfWatcherReport(url: string) {
	try {
		// Get the tab with the PDF document
		const [tab] = await chrome.tabs.query({
			active: true,
			currentWindow: true
		});

		if (!tab || !tab.id) {
			throw new Error('Could not find tab with PDF document');
		}
		return new Promise((resolve, reject) => {
			chrome.tabs.sendMessage(tab.id, { type: 'GENERATE_PDF_REPORT' }, (response) => {
				if (chrome.runtime.lastError) {
					reject(chrome.runtime.lastError);
				} else if (response && response.error) {
					reject(new Error(response.error));
				} else {
					resolve({
						...response
					});
				}
			});
		});
	} catch (error) {
		console.error('Error requesting PDF report:', error);
		return {
			error: String(error)
		};
	}
}

/**
 * Requests a report from YouTube Watcher for the given URL, including the current video timestamp
 */
async function requestYouTubeWatcherReport(url: string) {
	try {
		// Get the video ID from the URL
		const videoId = new URL(url).searchParams.get('v');
		if (!videoId) {
			throw new Error('Could not extract video ID from URL');
		}

		// Get the tab with the YouTube video
		const [tab] = await chrome.tabs.query({
			active: true,
			currentWindow: true
		});

		if (!tab || !tab.id) {
			throw new Error('Could not find tab with YouTube video');
		}

		// Send a message to the content script in the tab
		return new Promise((resolve, reject) => {
			chrome.tabs.sendMessage(tab.id, { type: 'GENERATE_YOUTUBE_REPORT', videoId }, (response) => {
				if (chrome.runtime.lastError) {
					reject(chrome.runtime.lastError);
				} else if (response && response.error) {
					reject(new Error(response.error));
				} else {
					// Process the response - it should now include timestamp
					// If no timestamp is provided or user is not watching video, default to -1
					const timestamp = response?.timestamp !== undefined ? response.timestamp : -1;

					resolve({
						...response,
						timestamp
					});
				}
			});
		});
	} catch (error) {
		console.error('Error requesting YouTube report:', error);
		// Return a response with timestamp -1 to indicate failure or no video playing
		return {
			error: String(error),
			timestamp: -1,
			isWatchingVideo: false
		};
	}
}

/**
 * Requests a report from Article Watcher for the given URL
 */
async function requestArticleWatcherReport(url: string) {
	try {
		// Get the tab with the article
		const [tab] = await chrome.tabs.query({
			active: true,
			currentWindow: true
		});

		if (!tab || !tab.id) {
			throw new Error('Could not find tab with article');
		}

		// Send a message to the content script in the tab
		return new Promise((resolve, reject) => {
			chrome.tabs.sendMessage(tab.id, { type: 'GENERATE_ARTICLE_REPORT', url }, (response) => {
				if (chrome.runtime.lastError) {
					reject(chrome.runtime.lastError);
				} else if (response && response.error) {
					reject(new Error(response.error));
				} else {
					resolve({
						...response
					});
				}
			});
		});
	} catch (error) {
		console.error('Error requesting article report:', error);
		return {
			error: String(error)
		};
	}
}
