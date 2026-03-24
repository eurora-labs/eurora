import browser from 'webextension-polyfill';
import type { NativeResponse } from '../../models';

export type MessageType = 'NEW' | 'GENERATE_ASSETS' | 'GENERATE_SNAPSHOT';

export type BrowserObj = { type: string; [key: string]: unknown };

export type WatcherResponse = NativeResponse | void;

export abstract class Watcher<T> {
	public params: T;

	constructor(params: T) {
		this.params = params;
	}

	public listen(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
		sendResponse: (response?: any) => void,
	): boolean {
		switch (obj.type) {
			case 'NEW':
				this.wrapAsync(this.handleNew(obj, sender), sendResponse);
				return true;
			case 'GENERATE_ASSETS':
				this.wrapAsync(this.handleGenerateAssets(obj, sender), sendResponse);
				return true;
			case 'GENERATE_SNAPSHOT':
				this.wrapAsync(this.handleGenerateSnapshot(obj, sender), sendResponse);
				return true;
			default:
				return false;
		}
	}

	private wrapAsync(
		promise: Promise<WatcherResponse>,
		sendResponse: (r?: WatcherResponse) => void,
	) {
		promise
			.then((result) => sendResponse(result))
			.catch((error) => {
				const message = error instanceof Error ? error.message : String(error);
				sendResponse({ kind: 'Error', data: message });
			});
	}

	abstract handleNew(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse>;

	abstract handleGenerateAssets(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse>;

	abstract handleGenerateSnapshot(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse>;
}
