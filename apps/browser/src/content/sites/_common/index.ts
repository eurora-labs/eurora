import browser from 'webextension-polyfill';
import type { CommonBrowserMessage, WatcherParams } from './types.js';
import type { WatcherResponse } from '../../../shared/content/extensions/watchers/watcher';

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
		const icon_base64 = await this.getFavicon();
		const responseFrame = {
			kind: 'NativeMetadata',
			data: {
				url: window.location.href,
				icon_base64,
			},
		};
		return responseFrame;
	}

	private getFaviconUrl(): string | null {
		const selectors = [
			'link[rel="icon"]',
			'link[rel="shortcut icon"]',
			'link[rel="mask-icon"]',
			'link[rel="apple-touch-icon"]',
			'link[rel="apple-touch-icon-precomposed"]',
		];

		for (const sel of selectors) {
			const link = document.querySelector(sel) as HTMLLinkElement;
			if (link && link.href) {
				return link.href;
			}
		}

		try {
			return new URL('/favicon.ico', window.location.origin).href;
		} catch (_) {
			return null;
		}
	}

	private async getFavicon(): Promise<string> {
		try {
			const faviconUrl = this.getFaviconUrl();
			if (!faviconUrl) {
				console.warn('No favicon found');
				return '';
			}

			let response: Response;
			try {
				response = await fetch(faviconUrl, { credentials: 'include' });
			} catch (err) {
				console.error('Failed to fetch favicon', err);
				return '';
			}
			const blob = await response.blob();

			return await new Promise<string>((resolve) => {
				const reader = new FileReader();
				reader.onloadend = () => {
					const dataUrl = reader.result;
					if (typeof dataUrl !== 'string') {
						console.warn('Unexpected FileReader result');
						resolve('');
						return;
					}
					const base64 = dataUrl.split(',')[1] || '';
					resolve(base64);
				};
				reader.onerror = () => resolve('');
				reader.readAsDataURL(blob);
			});
		} catch (err) {
			console.error('Failed to read favicon', err);
			return '';
		}
	}
}

export function main() {
	const watcher = new CommonWatcher({});

	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
