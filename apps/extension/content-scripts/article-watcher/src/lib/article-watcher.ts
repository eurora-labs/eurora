import { Watcher } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import {
	createArticleAsset,
	createArticleSnapshot,
} from '@eurora/chrome-ext-shared/extensions/article/util';
import { NativeResponse } from '@eurora/chrome-ext-shared/models';
import { ArticleChromeMessage, type ArticleMessageType, type WatcherParams } from './types.js';
import { Readability } from '@mozilla/readability';

class ArticleWatcher extends Watcher<WatcherParams> {
	constructor(params: WatcherParams) {
		super(params);
	}

	public listen(
		obj: ArticleChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		const { type } = obj;

		let promise: Promise<any> | null = null;

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
				response();
		}

		promise?.then((result) => {
			response(result);
		});

		return true;
	}

	public async handleNew(
		obj: ArticleChromeMessage,
		sender: chrome.runtime.MessageSender,
	): Promise<any> {
		console.log('Article Watcher: New article detected');
		// Parse article on page load for caching
		const clone = document.cloneNode(true) as Document;
		const article = new Readability(clone).parse();
		console.log('Parsed article:', article);
	}

	public async handleGenerateAssets(
		obj: ArticleChromeMessage,
		sender: chrome.runtime.MessageSender,
	): Promise<any> {
		console.log('Generating article report for URL:', window.location.href);
		const result = createArticleAsset(document);
		return result;
	}

	public async handleGenerateSnapshot(
		obj: ArticleChromeMessage,
		sender: chrome.runtime.MessageSender,
	): Promise<any> {
		const result = createArticleSnapshot(window);
		return result;
	}
}

(() => {
	console.log('Article Watcher content script loaded');

	const watcher = new ArticleWatcher({});

	// Parse article on page load
	window.addEventListener('load', () => {
		const clone = document.cloneNode(true) as Document;
		const article = new Readability(clone).parse();
		console.log('Parsed article on load:', article);
	});

	chrome.runtime.onMessage.addListener(watcher.listen.bind(watcher));
})();
