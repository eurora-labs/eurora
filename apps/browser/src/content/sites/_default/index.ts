import {
	createArticleAsset,
	createArticleSnapshot,
} from '../../../shared/content/extensions/article/util';
import { Watcher, type WatcherResponse } from '../../../shared/content/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import type { ArticleBrowserMessage, WatcherParams } from './types.js';

export class ArticleWatcher extends Watcher<WatcherParams> {
	constructor(params: WatcherParams) {
		super(params);
	}

	public listen(
		obj: ArticleBrowserMessage,
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
		_obj: ArticleBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		return { kind: 'Ok', data: null };
	}

	public async handleGenerateAssets(
		_obj: ArticleBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		return createArticleAsset(document);
	}

	public async handleGenerateSnapshot(
		_obj: ArticleBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		return createArticleSnapshot(window);
	}
}

export function main() {
	const watcher = new ArticleWatcher({});

	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
