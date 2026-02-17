import { Watcher, type WatcherResponse } from '../../../shared/content/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import type { TwitterBrowserMessage, WatcherParams } from './types.js';

import type {
	NativeTwitterAsset,
	NativeTwitterSnapshot,
	NativeTwitterTweet,
} from '../../../shared/content/bindings';

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
		obj: TwitterBrowserMessage,
		sender: browser.Runtime.MessageSender,
		response: (response?: WatcherResponse) => void,
	): boolean {
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
		_obj: TwitterBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		// Update current URL and page info
		this.params.currentUrl = window.location.href;
		this.params.pageTitle = document.title;

		// Get initial tweet data
		this.params.tweets = this.getTweetTexts();

		return { kind: 'Ok', data: null };
	}

	public async handleGenerateAssets(
		_obj: TwitterBrowserMessage,
		_sender: browser.Runtime.MessageSender,
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
		_obj: TwitterBrowserMessage,
		_sender: browser.Runtime.MessageSender,
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
