import browser from 'webextension-polyfill';
import type { NativeResponse } from '../../models';

export type MessageType = 'NEW' | 'COLLECT' | 'GENERATE_ASSETS' | 'GENERATE_SNAPSHOT';

export type BrowserObj = { type: string; [key: string]: unknown };

export type WatcherResponse = NativeResponse | void;

export interface CollectPayload {
	assets: NativeResponse | null;
	snapshot: NativeResponse | null;
}

const COLLECT_TIMEOUT_MS = 25_000;
const DEBOUNCE_MS = 500;

export abstract class Watcher<T> {
	public params: T;
	private pendingCollect: ((response?: WatcherResponse) => void) | null = null;
	private collectTimer: ReturnType<typeof setTimeout> | null = null;
	private observer: MutationObserver | null = null;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;
	private hasChange = false;

	constructor(params: T) {
		this.params = params;
	}

	public listen(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
		sendResponse: (response?: any) => void,
	): boolean {
		switch (obj.type) {
			case 'COLLECT':
				return this.handleCollect(sendResponse);
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

	private handleCollect(sendResponse: (response?: any) => void): boolean {
		if (this.hasChange) {
			this.hasChange = false;
			this.respondToCollect(sendResponse);
		} else {
			this.pendingCollect = sendResponse;
			this.collectTimer = setTimeout(() => {
				if (this.pendingCollect === sendResponse) {
					this.pendingCollect = null;
					this.collectTimer = null;
					sendResponse({ kind: 'NoChange', data: null });
				}
			}, COLLECT_TIMEOUT_MS);
		}
		return true;
	}

	private async respondToCollect(sendResponse: (response?: any) => void) {
		try {
			const [assets, snapshot] = await Promise.all([
				this.handleGenerateAssets({} as BrowserObj, {} as browser.Runtime.MessageSender),
				this.handleGenerateSnapshot({} as BrowserObj, {} as browser.Runtime.MessageSender),
			]);
			const payload: CollectPayload = {
				assets: assets ?? null,
				snapshot: snapshot ?? null,
			};
			sendResponse({ kind: 'CollectResponse', data: payload });
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			sendResponse({ kind: 'Error', data: message });
		}
	}

	protected onChangeDetected(): void {
		this.hasChange = true;
		if (this.pendingCollect) {
			const sendResponse = this.pendingCollect;
			this.pendingCollect = null;
			if (this.collectTimer) {
				clearTimeout(this.collectTimer);
				this.collectTimer = null;
			}
			this.hasChange = false;
			this.respondToCollect(sendResponse);
		}
	}

	public startChangeDetection(): void {
		this.stopChangeDetection();
		const target = this.getObserveTarget();
		if (!target) return;

		this.observer = new MutationObserver(() => {
			this.debouncedChange();
		});

		this.observer.observe(target, this.getObserveOptions());
	}

	public stopChangeDetection(): void {
		if (this.observer) {
			this.observer.disconnect();
			this.observer = null;
		}
		if (this.debounceTimer) {
			clearTimeout(this.debounceTimer);
			this.debounceTimer = null;
		}
	}

	public triggerInitialChange(): void {
		setTimeout(() => this.onChangeDetected(), 100);
	}

	protected getObserveTarget(): Node | null {
		return document.body;
	}

	protected getObserveOptions(): MutationObserverInit {
		return {
			childList: true,
			subtree: true,
			characterData: true,
		};
	}

	private debouncedChange(): void {
		if (this.debounceTimer) clearTimeout(this.debounceTimer);
		this.debounceTimer = setTimeout(() => {
			this.debounceTimer = null;
			this.onChangeDetected();
		}, DEBOUNCE_MS);
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
