import { TwitterParser } from './parser';
import { Watcher, type WatcherResponse } from '../../../shared/content/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import type { TwitterBrowserMessage, WatcherParams } from './types.js';

import type {
	NativeTwitterAsset,
	NativeTwitterSnapshot,
	NativeTwitterTweet,
	ParseResult,
} from '../../../shared/content/bindings';

export class TwitterWatcher extends Watcher<WatcherParams> {
	private parser = new TwitterParser();

	constructor(params: WatcherParams) {
		super(params);
	}

	private getTweets(result: ParseResult): NativeTwitterTweet[] {
		if (result.page === 'unsupported') return [];
		if (result.page === 'tweet') {
			const tweets: NativeTwitterTweet[] = [];
			if (result.data.tweet) tweets.push(result.data.tweet);
			tweets.push(...result.data.replies);
			return tweets;
		}
		return result.data.tweets;
	}

	public async handleNew(
		_obj: TwitterBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		const result = this.parser.parse(document);

		this.params.currentUrl = window.location.href;
		this.params.pageTitle = document.title;
		this.params.tweets = this.getTweets(result);

		return { kind: 'Ok', data: null };
	}

	public async handleGenerateAssets(
		_obj: TwitterBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		try {
			const result = this.parser.parse(document);

			const reportData: NativeTwitterAsset = {
				url: window.location.href,
				title: document.title,
				result,
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
			const result = this.parser.parse(document);
			const currentTweets = this.getTweets(result);

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

let initialized = false;

export function main() {
	if (initialized) return;
	initialized = true;

	const watcher = new TwitterWatcher({
		currentUrl: window.location.href,
		pageTitle: document.title,
		tweets: [],
	});

	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
	watcher.startChangeDetection();
	watcher.triggerInitialChange();
}
