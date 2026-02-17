import browser from 'webextension-polyfill';
import type { NativeResponse } from '../../models';

export type MessageType = 'NEW' | 'GENERATE_ASSETS' | 'GENERATE_SNAPSHOT';

export type BrowserObj = { type: MessageType; [key: string]: unknown };

export type WatcherResponse = NativeResponse | void;

export abstract class Watcher<T> {
	public params: T;

	constructor(params: T) {
		this.params = params;
	}

	abstract listen(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
		response: (response?: WatcherResponse) => void,
	): boolean;

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
