import { Watcher, type WatcherResponse } from '../../../shared/content/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import type { GoogleDocsBrowserMessage, WatcherParams } from './types.js';

export class GoogleDocsWatcher extends Watcher<WatcherParams> {
	constructor(params: WatcherParams) {
		super(params);
	}

	public async handleNew(
		_obj: GoogleDocsBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		return { kind: 'Ok', data: null };
	}
}

let initialized = false;

export function main() {
	if (initialized) return;
	initialized = true;

	const watcher = new GoogleDocsWatcher({});
	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
