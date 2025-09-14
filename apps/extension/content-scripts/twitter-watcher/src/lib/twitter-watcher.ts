import { Watcher } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import { TwitterChromeMessage, type TwitterMessageType, type WatcherParams } from './types.js';

import {
	NativeTwitterAsset,
	NativeTwitterSnapshot,
	NativeTwitterTweet,
} from '@eurora/chrome-ext-shared/bindings';

class TwitterWatcher extends Watcher<WatcherParams> {
	constructor(params: WatcherParams) {
		super(params);
	}

	private getTweetTexts(): NativeTwitterTweet[] {
		const tweets: NativeTwitterTweet[] = [];

		// Find all tweet elements with data-testid="tweetText"
		const tweetElements = document.querySelectorAll('[data-testid="tweetText"]');

		tweetElements.forEach((tweetElement) => {
			// Get the span child that contains the actual text
			const spanElement = tweetElement.querySelector('span');
			if (spanElement && spanElement.textContent) {
				tweets.push({
					text: spanElement.textContent.trim(),
					timestamp: null,
					author: null,
				});
			}
		});

		return tweets;
	}

	public listen(
		obj: TwitterChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		const { type } = obj;

		switch (type) {
			case 'NEW':
				this.handleNew(obj, sender, response);
				break;
			case 'GENERATE_ASSETS':
				this.handleGenerateAssets(obj, sender, response);
				break;
			case 'GENERATE_SNAPSHOT':
				this.handleGenerateSnapshot(obj, sender, response);
				break;
			case 'TEST':
				this.handleTest(obj, sender, response);
				break;
		}
	}

	public handleNew(
		obj: TwitterChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		// Update current URL and page info
		this.params.currentUrl = window.location.href;
		this.params.pageTitle = document.title;

		// Get initial tweet data
		this.params.tweets = this.getTweetTexts();

		console.log('Twitter watcher initialized:', {
			url: this.params.currentUrl,
			title: this.params.pageTitle,
			tweetCount: this.params.tweets.length,
		});
	}

	public handleTest(
		obj: TwitterChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		const tweets = this.getTweetTexts();
		console.log('Twitter test - found tweets:', tweets);
		response({ tweets, count: tweets.length });
	}

	public handleGenerateAssets(
		obj: TwitterChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		try {
			// Get current tweet texts
			const currentTweets = this.getTweetTexts();

			const reportData: NativeTwitterAsset = {
				url: window.location.href,
				title: document.title,
				tweets: currentTweets,
				timestamp: new Date().toISOString(),
			};

			response({ kind: 'NativeTwitterAsset', data: reportData });
			return true;
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : String(error);
			const contextualError = `Failed to generate Twitter assets for ${window.location.href}: ${errorMessage}`;
			console.error('Error generating Twitter report:', {
				url: window.location.href,
				error: errorMessage,
				stack: error instanceof Error ? error.stack : undefined,
			});
			response({
				success: false,
				error: contextualError,
				context: {
					url: window.location.href,
					timestamp: new Date().toISOString(),
				},
			});
		}

		return true; // Important: indicates we'll send response asynchronously
	}

	public handleGenerateSnapshot(
		obj: TwitterChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		console.log('Generating snapshot for Twitter page');

		try {
			const currentTweets = this.getTweetTexts();

			const reportData: NativeTwitterSnapshot = {
				tweets: currentTweets,
				timestamp: new Date().toISOString(),
			};

			response({ kind: 'NativeTwitterSnapshot', data: reportData });

			return true;
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : String(error);
			console.error('Error generating Twitter snapshot:', {
				url: window.location.href,
				error: errorMessage,
			});
			response({
				success: false,
				error: `Failed to generate Twitter snapshot: ${errorMessage}`,
			});
		}

		return true;
	}
}

(() => {
	const watcher = new TwitterWatcher({
		currentUrl: window.location.href,
		pageTitle: document.title,
		tweets: [],
	});

	chrome.runtime.onMessage.addListener(watcher.listen.bind(watcher));
})();
