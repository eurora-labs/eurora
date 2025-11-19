import browser from 'webextension-polyfill';
import type { Frame, RequestFrame, ResponseFrame } from '../content/bindings.js';
import { getCurrentTabIcon } from './tabs.js';
import { handleMessage } from './messaging.js';
import { onUpdated, onActivated } from './focus-tracker.js';

const host = 'com.eurora.app';
const connectTimeout = 5000;
let nativePort: browser.Runtime.Port | null = null;

export function startNativeMessenger() {
	nativePort = browser.runtime.connectNative(host);
	nativePort.onDisconnect.addListener(onNativePortDisconnect);
	nativePort?.onMessage.addListener(onNativePortMessage);
	browser.tabs.onUpdated.addListener(async (tabId, changeInfo, tab) => {
		if (!nativePort) return;

		await onUpdated(tabId, changeInfo, tab, nativePort);
	});

	browser.tabs.onActivated.addListener(async (activeInfo) => {
		if (!nativePort) return;

		await onActivated(activeInfo.tabId, nativePort);
	});

	console.log('Native messaging service worker registered');
}

function onNativePortDisconnect(port: browser.Runtime.Port) {
	const error = port.error;
	console.error('Native port disconnected:', error?.message || 'Unknown error');
	nativePort = null;

	// Try to reconnect after a delay
	setTimeout(() => {
		connect();
		console.log('Reconnected to native host');
	}, connectTimeout);
}

function connect() {
	nativePort = browser.runtime.connectNative(host);
	nativePort.onDisconnect.addListener(onNativePortDisconnect);
	nativePort?.onMessage.addListener(onNativePortMessage);
}

async function onNativePortMessage(message: unknown, sender: browser.Runtime.Port) {
	// Assert type here
	const frame = message as Frame;
	const kind = frame.kind;
	if (!kind) {
		console.error('Invalid frame kind');
		throw new Error('Invalid frame kind');
	}

	console.log('kind: ', kind);
	if ('Request' in kind) {
		sender.postMessage(await onRequestFrame(kind.Request));
	} else if ('Response' in kind) {
		sender.postMessage(await onResponseFrame(kind.Response));
	} else if ('Event' in kind) {
		throw new Error('Event frames are not supported');
	} else if ('Error' in kind) {
		throw new Error('Error frames are not supported');
	} else if ('Cancel' in kind) {
		throw new Error('Cancel frames are not supported');
	}

	return true;
}

async function onRequestFrame(frame: RequestFrame): Promise<Frame> {
	switch (frame.action) {
		case 'GET_METADATA':
			return await onActionMetadata(frame);
		default:
			const response = await handleMessage(frame.action);
			console.log('Finished responding to ', frame.action, ': ', response);
			const responseFrame: ResponseFrame = {
				id: frame.id,
				action: frame.action,
				payload: JSON.stringify(response),
			};
			return {
				kind: {
					Response: responseFrame,
				},
			} as Frame;
	}
}

async function onResponseFrame(frame: ResponseFrame): Promise<Frame> {
	throw new Error('Not implemented');
}

async function onActionMetadata(frame: RequestFrame): Promise<Frame> {
	const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
	const iconBase64 = await getCurrentTabIcon(activeTab);
	console.log('Tab metadata:', { url: activeTab.url, icon_base64: iconBase64 });

	const response: ResponseFrame = {
		id: frame.id,
		action: frame.action,
		payload: JSON.stringify({
			kind: 'NativeMetadata',
			data: {
				url: activeTab.url,
				icon_base64: iconBase64,
			},
		}),
	};

	return {
		kind: {
			Response: response,
		},
	} as Frame;
}

// export class NativeMessenger {
// 	private host: string;
// 	private nativePort: browser.Runtime.Port | null = null;
// 	private connectTimeout: number;

// 	constructor(host: string = 'com.eurora.app', connectTimeout: number = 5000) {
// 		this.host = host;
// 		this.connectTimeout = connectTimeout;
// 	}

// 	public startNativeMessenger() {
// 		this.connect();
// 		browser.tabs.onUpdated.addListener(async (tabId, changeInfo, tab) => {
// 			if (!this.nativePort) return;

// 			await onUpdated(tabId, changeInfo, tab, this.nativePort);
// 		});

// 		browser.tabs.onActivated.addListener(async (activeInfo) => {
// 			if (!this.nativePort) return;

// 			await onActivated(activeInfo.tabId, this.nativePort);
// 		});

// 		console.log('Native messaging service worker registered');
// 	}

// 	private connect() {
// 		this.nativePort = browser.runtime.connectNative(this.host);
// 		this.nativePort.onDisconnect.addListener(this.onNativePortDisconnect);
// 		this.nativePort?.onMessage.addListener(this.onNativePortMessage);
// 	}

// 	private async onNativePortMessage(message: unknown, sender: browser.Runtime.Port) {
// 		// Assert type here
// 		const frame = message as Frame;
// 		const kind = frame.kind;
// 		if (!kind) {
// 			console.error('Invalid frame kind');
// 			throw new Error('Invalid frame kind');
// 		}

// 		console.log('kind: ', kind);
// 		console.log('this: ', this);
// 		if ('Request' in kind) {
// 			sender.postMessage(await this.onRequestFrame(kind.Request));
// 		} else if ('Response' in kind) {
// 			sender.postMessage(await this.onResponseFrame(kind.Response));
// 		} else if ('Event' in kind) {
// 			throw new Error('Event frames are not supported');
// 		} else if ('Error' in kind) {
// 			throw new Error('Error frames are not supported');
// 		} else if ('Cancel' in kind) {
// 			throw new Error('Cancel frames are not supported');
// 		}

// 		return true;
// 	}

// 	private async onRequestFrame(frame: RequestFrame): Promise<Frame> {
// 		switch (frame.action) {
// 			case 'GET_METADATA':
// 				return await this.onActionMetadata(frame);
// 			default:
// 				const response = await handleMessage(frame.action);
// 				console.log('Finished responding to ', frame.action, ': ', response);
// 				const responseFrame: ResponseFrame = {
// 					id: frame.id,
// 					action: frame.action,
// 					payload: JSON.stringify(response),
// 				};
// 				return {
// 					kind: {
// 						Response: responseFrame,
// 					},
// 				} as Frame;
// 		}
// 	}

// 	private async onResponseFrame(frame: ResponseFrame): Promise<Frame> {
// 		throw new Error('Not implemented');
// 	}

// 	private async onActionMetadata(frame: RequestFrame): Promise<Frame> {
// 		const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
// 		const iconBase64 = await getCurrentTabIcon(activeTab);
// 		console.log('Tab metadata:', { url: activeTab.url, icon_base64: iconBase64 });

// 		const response: ResponseFrame = {
// 			id: frame.id,
// 			action: frame.action,
// 			payload: JSON.stringify({
// 				kind: 'NativeMetadata',
// 				data: {
// 					url: activeTab.url,
// 					icon_base64: iconBase64,
// 				},
// 			}),
// 		};

// 		return {
// 			kind: {
// 				Response: response,
// 			},
// 		} as Frame;
// 	}

// 	private async onNativePortDisconnect(port: browser.Runtime.Port) {
// 		const error = port.error;
// 		console.error('Native port disconnected:', error?.message || 'Unknown error');
// 		this.nativePort = null;

// 		// Try to reconnect after a delay
// 		setTimeout(() => {
// 			this.connect();
// 			console.log('Reconnected to native host');
// 		}, this.connectTimeout);
// 	}
// }
