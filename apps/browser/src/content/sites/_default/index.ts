import { Watcher, type WatcherResponse } from '../../../shared/content/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import type { ArticleBrowserMessage, WatcherParams } from './types.js';

export class ArticleWatcher extends Watcher<WatcherParams> {
	constructor(params: WatcherParams) {
		super(params);
	}

	public async handleNew(
		_obj: ArticleBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		return { kind: 'Ok', data: null };
	}
}

let initialized = false;

export function main() {
	if (initialized) return;
	initialized = true;

	const watcher = new ArticleWatcher({});
	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}

export { main as mainDefault };
