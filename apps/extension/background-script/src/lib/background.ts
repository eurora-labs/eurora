// Background script for tab tracking and content script initialization
import {
	ContentScriptContext,
	YouTubeStrategy,
	ArticleStrategy,
	PdfStrategy
} from './strategies/index.js';

// Create and configure the content script context
const contentScriptContext = new ContentScriptContext();

// Register all available strategies
contentScriptContext.registerStrategy(new YouTubeStrategy());
contentScriptContext.registerStrategy(new PdfStrategy());
// Register the article strategy as the default strategy
contentScriptContext.registerStrategy(new ArticleStrategy(), true);

// Listen for tab updates
chrome.tabs.onUpdated.addListener(function (tabId, changeInfo, tab) {
	// Only process when the page is fully loaded and has a URL
	if (changeInfo.status !== 'complete' || !tab.url) return;

	// Process the tab with the appropriate strategy
	contentScriptContext.processTab(tabId, tab.url);
});

// Lifecycle handlers
chrome.runtime.onInstalled.addListener((details) => {
	console.log('Extension installed or updated:', details.reason);
});

console.log('Background script initialized with Strategy Pattern');
