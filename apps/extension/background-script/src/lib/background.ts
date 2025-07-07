// Background script for tab tracking and content script initialization
// import {
// 	ContentScriptContext,
// 	YouTubeStrategy,
// 	ArticleStrategy,
// 	PdfStrategy
// } from './strategies/index.js';

// Create and configure the content script context
// const contentScriptContext = new ContentScriptContext();

// // Register all available strategies
// contentScriptContext.registerStrategy(new YouTubeStrategy());
// contentScriptContext.registerStrategy(new PdfStrategy());
// // Register the article strategy as the default strategy
// contentScriptContext.registerStrategy(new ArticleStrategy(), true);

// Listen for tab updates

// Lifecycle handlers
chrome.runtime.onInstalled.addListener((details) => {
	console.log('Extension installed or updated:', details.reason);
	chrome.tabs.onUpdated.addListener(function (tabId, changeInfo, tab) {
		if (changeInfo.status === 'complete' && tab.url) {
			chrome.tabs.sendMessage(
				tabId,
				{
					type: 'NEW',
					value: tab.url,
				},
				(response) => {
					console.log('Received response from content script:', response);
				},
			);
		}
	});
});

console.log('Background script initialized with Strategy Pattern');
