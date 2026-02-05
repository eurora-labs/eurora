import type { WatcherResponse } from '@eurora/browser-shared/content/extensions/watchers/watcher';
import browser from 'webextension-polyfill';
import type { CommonBrowserMessage, WatcherParams } from './types.js';

export class CommonWatcher {
	public params: WatcherParams;

	constructor(params: WatcherParams) {
		this.params = params;
	}

	public listen(
		obj: CommonBrowserMessage,
		_sender: browser.Runtime.MessageSender,
		response: (response?: WatcherResponse) => void,
	): boolean {
		const { type } = obj;

		let promise: Promise<WatcherResponse>;

		switch (type) {
			case 'GET_METADATA':
				promise = this.handleGetMetadata(obj, _sender);
				break;
			default:
				// Return false for unhandled message types to allow other listeners to process
				return false;
		}

		promise
			.then((result) => {
				response(result);
			})
			.catch((error) => {
				const message = error instanceof Error ? error.message : String(error);
				console.error('Common watcher failed', { error });
				response({ kind: 'Error', data: message });
			});

		return true;
	}

	public async handleGetMetadata(
		_obj: CommonBrowserMessage,
		_sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse> {
		// TODO: Implement GET_METADATA functionality
		return { kind: 'Ok', data: null };
	}
}

export function main() {
	const watcher = new CommonWatcher({});

	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
