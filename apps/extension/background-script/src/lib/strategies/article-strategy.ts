import { ContentScriptStrategy } from './content-script-strategy.js';
import { isArticlePage } from '../utils/page-detection.js';

/**
 * Article Content Script Strategy
 * Handles article pages
 */
export class ArticleStrategy implements ContentScriptStrategy {
	/**
	 * Check if this strategy can handle the given URL
	 * @param url The URL to check
	 * @returns True if this is an article page
	 */
	canHandle(url: string): boolean {
		return isArticlePage(url);
	}

	/**
	 * Initialize the article content script for the given tab
	 * @param tabId The ID of the tab
	 * @param url The URL of the tab
	 */
	initialize(tabId: number, url: string): void {
		console.log('Article page detected, initializing article watcher', tabId);

		chrome.tabs.sendMessage(tabId, {
			type: 'NEW_ARTICLE'
		});
	}
}
