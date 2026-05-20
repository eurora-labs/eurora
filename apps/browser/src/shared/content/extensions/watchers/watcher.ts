import {
	handleGetAccessibilityTree,
	handleGetPageMetadata,
	handleGetReadabilityArticle,
	handleGetSelectedText,
	handleInsertText,
	handleListFormInputs,
	handleListLinks,
	handleQuerySelector,
} from '../web';
import browser from 'webextension-polyfill';
import type { NativeResponse } from '../../models';

export type MessageType =
	| 'NEW'
	| 'GET_PAGE_METADATA'
	| 'GET_ACCESSIBILITY_TREE'
	| 'GET_READABILITY_ARTICLE'
	| 'GET_SELECTED_TEXT'
	| 'QUERY_SELECTOR'
	| 'LIST_LINKS'
	| 'LIST_FORM_INPUTS'
	| 'INSERT_TEXT';

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
	//
	// The Promise resolves to `unknown` so subclasses can return typed
	// payloads (e.g. adapter-driven tool replies) that don't conform to
	// `NativeResponse` — the cross-process message bus only requires the
	// value to be JSON-serializable.
	public listen(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
	): Promise<unknown> | false {
		switch (obj.type) {
			case 'NEW':
				return this.guard(this.handleNew(obj, sender));
			// Generic web tools — available on every page through the base
			// watcher, regardless of which site-specific subclass is mounted.
			// Site-specific overrides should fall through to `super.listen`
			// on no-match so these stay reachable.
			case 'GET_PAGE_METADATA':
				return this.guard(handleGetPageMetadata());
			case 'GET_ACCESSIBILITY_TREE':
				return this.guard(handleGetAccessibilityTree(obj));
			case 'GET_READABILITY_ARTICLE':
				return this.guard(handleGetReadabilityArticle());
			case 'GET_SELECTED_TEXT':
				return this.guard(handleGetSelectedText());
			case 'QUERY_SELECTOR':
				return this.guard(handleQuerySelector(obj));
			case 'LIST_LINKS':
				return this.guard(handleListLinks(obj));
			case 'LIST_FORM_INPUTS':
				return this.guard(handleListFormInputs(obj));
			case 'INSERT_TEXT':
				return this.guard(handleInsertText(obj));
			default:
				return false;
		}
	}

	/// Convert a rejected handler promise into the `{kind: 'Error'}`
	/// envelope the bridge expects. Subclasses that override `listen` to
	/// add new message types should route their handler promises through
	/// this so a thrown error doesn't bubble out as a raw `sendMessage`
	/// rejection (which the bridge interprets as a tab-gone signal).
	///
	/// The success type is preserved verbatim — the message bus
	/// serializes whatever shape the handler returns, including flat
	/// typed payloads that aren't `NativeResponse`-shaped (e.g. the
	/// adapter-driven YouTube tool replies, and the structured
	/// `{kind: 'Error', code: 'SAFETY_VIOLATION', …}` envelope that
	/// `insert_text` emits for safety-contract violations).
	protected async guard<T>(promise: Promise<T>): Promise<T | NativeResponse> {
		return await promise.catch((error) => {
			const message = error instanceof Error ? error.message : String(error);
			return { kind: 'Error', data: message } as NativeResponse;
		});
	}

	abstract handleNew(
		obj: BrowserObj,
		sender: browser.Runtime.MessageSender,
	): Promise<WatcherResponse>;
}
