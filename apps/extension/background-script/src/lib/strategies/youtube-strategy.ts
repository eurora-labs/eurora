import { ContentScriptStrategy } from './content-script-strategy.js';
import { isYouTubeVideoUrl } from '../utils/url-helpers.js';

/**
 * YouTube Content Script Strategy
 * Handles YouTube video pages
 */
export class YouTubeStrategy implements ContentScriptStrategy {
	/**
	 * Check if this strategy can handle the given URL
	 * @param url The URL to check
	 * @returns True if this is a YouTube video URL
	 */
	canHandle(url: string): boolean {
		return isYouTubeVideoUrl(url);
	}

	/**
	 * Initialize the YouTube content script for the given tab
	 * @param tabId The ID of the tab
	 * @param url The URL of the tab
	 */
	initialize(tabId: number, url: string): void {
		console.log('YouTube page loaded, sending message to tab', tabId);

		const queryParameters = url.split('?')[1];
		const urlParameters = new URLSearchParams(queryParameters);

		chrome.tabs.sendMessage(tabId, {
			type: 'NEW',
			videoId: urlParameters.get('v')
		});
	}
}
