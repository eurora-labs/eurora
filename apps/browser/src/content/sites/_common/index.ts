import {
	collectIconCandidatesFromLinks,
	originFallbackCandidate,
	resolveBestCandidate,
	type IconCandidate,
	type IconLinkRecord,
} from '../../../shared/background/favicon-ranker';
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
		const icon_base64 = await resolveDocumentFavicon();
		return {
			kind: 'NativeMetadata',
			data: {
				url: window.location.href,
				icon_base64,
				title: document.title || null,
			},
		};
	}
}

async function resolveDocumentFavicon(): Promise<string> {
	const records: IconLinkRecord[] = Array.from(
		document.querySelectorAll<HTMLLinkElement>('link[rel]'),
	)
		.filter((link) => !!link.href)
		.map((link) => ({
			href: link.href,
			rel: link.rel || '',
			type: link.type || '',
			sizes: link.getAttribute('sizes') || '',
		}));

	const candidates: IconCandidate[] = collectIconCandidatesFromLinks(records);

	const fallback = originFallbackCandidate(window.location.href, candidates.length);
	if (fallback) candidates.push(fallback);

	return await resolveBestCandidate(candidates);
}

let initialized = false;

export function main() {
	if (initialized) return;
	initialized = true;

	const watcher = new CommonWatcher({});
	browser.runtime.onMessage.addListener(watcher.listen.bind(watcher));
}
