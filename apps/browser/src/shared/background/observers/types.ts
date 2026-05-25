import type browser from 'webextension-polyfill';

/**
 * A description of one context key the background tracks. Each observer
 * is responsible for:
 *
 *   - classifying the currently-active tab into a per-key state (or
 *     `null` when this key shouldn't be active);
 *   - comparing two states so the dispatcher can skip republishing when
 *     nothing meaningful changed;
 *   - shaping the wire payload sent on `CONTEXT_ACTIVATED`.
 *
 * Tool dispatch is **not** an observer concern — tools are content-script
 * driven; each tab's site bundle owns its own tool list and dispatch via
 * the `Watcher` interface in `shared/content/tools/types`.
 *
 * Adding a new context key (X, Google Docs, …) is a matter of writing
 * a new `ObserverSpec` and appending it to the dispatcher's list — no
 * other code in this directory changes.
 */
export interface ObserverSpec<S> {
	/**
	 * Context key emitted in `CONTEXT_ACTIVATED`/`CONTEXT_DEACTIVATED`
	 * payloads. Must match the server-side per-key formatter.
	 */
	readonly key: string;

	/**
	 * Resolve the active tab to a per-key state, or `null` when this key
	 * shouldn't be active. May read tab.url / tab.title only — the
	 * background script has no access to page content.
	 */
	classify(tab: browser.Tabs.Tab | undefined): S | null;

	/**
	 * Equality predicate for two non-null states. Used to suppress
	 * republishing when an `activated`/`updated` event arrives for the
	 * same tab and same observable state (e.g. a YouTube watch page
	 * being re-focused without changing videos).
	 */
	sameState(a: S, b: S): boolean;

	/**
	 * Shape the wire payload sent inside `CONTEXT_ACTIVATED`. The
	 * envelope (`key`, `origin`) is added by the dispatcher.
	 */
	activatedPayload(state: S): Record<string, unknown>;

	/**
	 * The tab the state belongs to. Used by the dispatcher to deactivate
	 * a state when its tab is removed.
	 */
	tabIdOf(state: S): number;

	/**
	 * Origin envelope the dispatcher emits alongside the activation
	 * payload. Extracted from the state so each observer can decide what
	 * window/page coordinates make sense for it.
	 */
	originOf(state: S): {
		tab_id: number;
		window_id: string;
		page_url: string;
	};
}
