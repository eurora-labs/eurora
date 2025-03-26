// Background script for tab tracking and content script initialization
import { isYouTubeVideoUrl } from './utils/url-helpers.js';
import { isArticlePage } from './utils/page-detection.ts';

chrome.tabs.onUpdated.addListener(function (tabId, changeInfo, tab) {
	if (changeInfo.status !== 'complete' || !tab.url) return;

	if (tab.url.includes('youtube.com/watch')) {
		const queryParameters = tab.url.split('?')[1];
		const urlParameters = new URLSearchParams(queryParameters);

		console.log('YouTube page loaded, sending message to tab', tabId);

		chrome.tabs.sendMessage(tabId, {
			type: 'NEW',
			videoId: urlParameters.get('v')
		});
	} else if (isArticlePage(tab.url)) {
		console.log('Article page detected, initializing article watcher', tabId);

		chrome.tabs.sendMessage(tabId, {
			type: 'NEW_ARTICLE'
		});
	}
});

// Lifecycle handlers
chrome.runtime.onInstalled.addListener((details) => {
	console.log('Extension installed or updated:', details.reason);
});

console.log('Background script initialized');
