export type MessageType = 'NEW' | 'GENERATE_ASSETS' | 'GENERATE_SNAPSHOT';

export type ChromeMessage = {
	message: any & { type: MessageType };
	sender: chrome.runtime.MessageSender;
	response: (response?: any) => void;
};

export abstract class Watcher<T> {
	public params: T;

	constructor(params: T) {
		this.params = params;
	}

	abstract listen(message: ChromeMessage): void;
	abstract generateAssets<T>(): Promise<T>;
	abstract generateSnapshot<T>(): Promise<T>;
}
