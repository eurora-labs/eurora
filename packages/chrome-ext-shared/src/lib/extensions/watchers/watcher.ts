import type { NativeResponse } from '$lib/models';

export type MessageType = 'NEW' | 'GENERATE_ASSETS' | 'GENERATE_SNAPSHOT';

export type ChromeObj = { type: MessageType; [key: string]: unknown };

export type WatcherResponse = NativeResponse | void;

export abstract class Watcher<T> {
	public params: T;

	constructor(params: T) {
		this.params = params;
	}

	abstract listen(
		obj: ChromeObj,
		sender: chrome.runtime.MessageSender,
		response: (response?: WatcherResponse) => void,
	): void;

	abstract handleNew(
		obj: ChromeObj,
		sender: chrome.runtime.MessageSender,
	): Promise<WatcherResponse>;

	abstract handleGenerateAssets(
		obj: ChromeObj,
		sender: chrome.runtime.MessageSender,
	): Promise<WatcherResponse>;

	abstract handleGenerateSnapshot(
		obj: ChromeObj,
		sender: chrome.runtime.MessageSender,
	): Promise<WatcherResponse>;
}
