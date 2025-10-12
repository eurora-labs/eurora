import {
	Watcher,
	type WatcherResponse,
} from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import {
	createArticleAsset,
	createArticleSnapshot,
} from '@eurora/chrome-ext-shared/extensions/article/util';
import { ArticleChromeMessage, type WatcherParams } from './types.js';

class ArticleWatcher extends Watcher<WatcherParams> {
	constructor(params: WatcherParams) {
		super(params);
	}

	public listen(
		obj: ArticleChromeMessage,
		sender: chrome.runtime.MessageSender,
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
		_obj: ArticleChromeMessage,
		_sender: chrome.runtime.MessageSender,
	): Promise<WatcherResponse> {
		console.log('Article Watcher: New article detected');
	}

	public async handleGenerateAssets(
		_obj: ArticleChromeMessage,
		_sender: chrome.runtime.MessageSender,
	): Promise<WatcherResponse> {
		return createArticleAsset(document);
	}

	public async handleGenerateSnapshot(
		_obj: ArticleChromeMessage,
		_sender: chrome.runtime.MessageSender,
	): Promise<WatcherResponse> {
		return createArticleSnapshot(window);
	}
}

export function main() {
	const watcher = new ArticleWatcher({});

	chrome.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
