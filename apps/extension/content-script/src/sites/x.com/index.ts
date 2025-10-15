import {
	Watcher,
	type WatcherResponse,
} from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import type { TwitterChromeMessage, WatcherParams } from './types.js';

import type {
	NativeTwitterAsset,
	NativeTwitterSnapshot,
	NativeTwitterTweet,
} from '@eurora/chrome-ext-shared/bindings';

export class TwitterWatcher extends Watcher<WatcherParams> {
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
		sender: browser.Runtime.MessageSender,
		response: (response?: WatcherResponse) => void,
	) {
		const { type } = obj;
		let promise: Promise<WatcherResponse>;

		switch (type) {
			case 'NEW':
				promise = this.handleNew(obj, sender);
				break;
			case 'GENERATE_ASSETS':
				promise = this.handleGenerateAssets(obj, sender);
				break;
			case 'GENERATE_SNAPSHOT':
				promise = this.handleGenerateSnapshot(obj, sender);
				break;
			default:
				response({ kind: 'Error', data: 'Invalid message type' });
				return false;
		}

		promise.then((result) => {
			response(result);
		});

		return true;
	}

	public async handleNew(
		obj: TwitterChromeMessage,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
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

	public async handleGenerateAssets(
		obj: TwitterChromeMessage,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		try {
			// Get current tweet texts
			const currentTweets = this.getTweetTexts();

			const reportData: NativeTwitterAsset = {
				url: window.location.href,
				title: document.title,
				tweets: currentTweets,
				timestamp: new Date().toISOString(),
			};

			return { kind: 'NativeTwitterAsset', data: reportData };
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : String(error);
			const contextualError = `Failed to generate Twitter assets for ${window.location.href}: ${errorMessage}`;

			console.error('Error generating Twitter report:', {
				url: window.location.href,
				error: errorMessage,
				stack: error instanceof Error ? error.stack : undefined,
			});

			return {
				kind: 'Error',
				data: contextualError,
			};
		}
	}

	public async handleGenerateSnapshot(
		obj: TwitterChromeMessage,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		try {
			const currentTweets = this.getTweetTexts();

			const reportData: NativeTwitterSnapshot = {
				tweets: currentTweets,
				timestamp: new Date().toISOString(),
			};

			return { kind: 'NativeTwitterSnapshot', data: reportData };
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : String(error);

			console.error('Error generating Twitter snapshot:', {
				url: window.location.href,
				error: errorMessage,
			});

			return {
				kind: 'Error',
				data: `Failed to generate Twitter snapshot: ${errorMessage}`,
			};
		}
	}
}

export function main() {
	const watcher = new TwitterWatcher({
		currentUrl: window.location.href,
		pageTitle: document.title,
		tweets: [],
	});

	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
