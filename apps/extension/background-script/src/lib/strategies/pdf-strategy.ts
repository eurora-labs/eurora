import { ContentScriptStrategy } from './content-script-strategy.js';
import { isPdfUrl } from '../utils/url-helpers.js';

/**
 * PDF Content Script Strategy
 * Handles PDF pages
 */
export class PdfStrategy implements ContentScriptStrategy {
	/**
	 * Check if this strategy can handle the given URL
	 * @param url The URL to check
	 * @returns True if this is a PDF page
	 */
	canHandle(url: string): boolean {
		return isPdfUrl(url);
	}

	/**
	 * Initialize the PDF content script for the given tab
	 * @param tabId The ID of the tab
	 * @param url The URL of the tab
	 */
	initialize(tabId: number, url: string): void {
		console.log('PDF page detected, initializing PDF watcher', tabId);

		chrome.tabs.sendMessage(tabId, {
			type: 'NEW_PDF'
		});
	}
}
