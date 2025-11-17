import browser from 'webextension-polyfill';

// Listen for tab updates
browser.tabs.onUpdated.addListener(function (tabId, changeInfo, tab) {
	// if (changeInfo.status === 'complete' && tab.url) {
	// 	browser.tabs
	// 		.sendMessage(tabId, {
	// 			type: 'NEW',
	// 			value: tab.url,
	// 		})
	// 		.then((response) => {
	// 			console.log('Received response from content script:', response);
	// 		})
	// 		.catch((error) => {
	// 			if (
	// 				error instanceof Error &&
	// 				error.message.includes('Could not establish connection')
	// 			) {
	// 				return;
	// 			}
	// 			console.log('Failed to relay NEW message to tab: ', tabId, error);
	// 		});
	// }
});

// Lifecycle handlers
browser.runtime.onInstalled.addListener((details) => {
	console.log('Extension installed or updated:', details.reason);
});

console.log('Background script initialized with Strategy Pattern');
