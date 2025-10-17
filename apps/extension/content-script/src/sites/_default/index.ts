import {
	Watcher,
	type WatcherResponse,
} from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import {
	createArticleAsset,
	createArticleSnapshot,
} from '@eurora/chrome-ext-shared/extensions/article/util';
import { ArticleChromeMessage, type WatcherParams } from './types.js';

export class ArticleWatcher extends Watcher<WatcherParams> {
	constructor(params: WatcherParams) {
		super(params);
	}

	public listen(
		obj: ArticleChromeMessage,
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

		promise
			.then((result) => {
				response(result);
			})
			.catch((error) => {
				const message = error instanceof Error ? error.message : String(error);
				console.error('Article watcher failed', { error });
				response({ kind: 'Error', data: message });
			});

		return true;
	}

	public async handleNew(
		_obj: ArticleChromeMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		console.log('Article Watcher: New article detected');
	}

	public async handleGenerateAssets(
		_obj: ArticleChromeMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		return createArticleAsset(document);
	}

	public async handleGenerateSnapshot(
		_obj: ArticleChromeMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		return createArticleSnapshot(window);
	}
}

export function main() {
	const watcher = new ArticleWatcher({});

	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
