export type MessageType = 'NEW' | 'GENERATE_ASSETS' | 'GENERATE_SNAPSHOT';

export type ChromeObj = { type: MessageType; [key: string]: unknown };

export abstract class Watcher<T> {
	public params: T;

	constructor(params: T) {
		this.params = params;
	}

	abstract listen(
		obj: ChromeObj,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	): void;

	abstract handleNew(obj: ChromeObj, sender: chrome.runtime.MessageSender): Promise<void>;

	abstract handleGenerateAssets(
		obj: ChromeObj,
		sender: chrome.runtime.MessageSender,
	): Promise<void>;

	abstract handleGenerateSnapshot(
		obj: ChromeObj,
		sender: chrome.runtime.MessageSender,
	): Promise<void>;
}
