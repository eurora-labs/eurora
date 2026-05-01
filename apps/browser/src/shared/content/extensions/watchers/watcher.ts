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

	// Return a Promise (not `true` + async sendResponse). Safari closes the
	// message port before async sendResponse callbacks fire, dropping the
	// reply; returning a Promise is the cross-browser pattern that
	// webextension-polyfill marshals correctly on all engines.
	public listen(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> | false {
		switch (obj.type) {
			case 'NEW':
				return this.guard(this.handleNew(obj, sender));
			case 'GENERATE_ASSETS':
				return this.guard(this.handleGenerateAssets(obj, sender));
			case 'GENERATE_SNAPSHOT':
				return this.guard(this.handleGenerateSnapshot(obj, sender));
			default:
				return false;
		}
	}

	private async guard(promise: Promise<WatcherResponse>): Promise<WatcherResponse> {
		return await promise.catch((error) => {
			const message = error instanceof Error ? error.message : String(error);
			return { kind: 'Error', data: message };
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
