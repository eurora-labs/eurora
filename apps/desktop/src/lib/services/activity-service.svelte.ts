import { ListenerBag } from '$lib/bindings/listeners.js';
import { unwrap } from '$lib/bindings/result.js';
import {
	commands,
	events,
	type SavedActivity,
	type SavedActivityEnded,
} from '$lib/bindings/specta.bindings.js';
import { InjectionToken } from '@eurora/shared/context';

/**
 * Page size for the initial snapshot fetch and every subsequent
 * `loadMore` call. The backend's `MAX_LIST_LIMIT` is 100, so this
 * stays well under the cap while keeping the rail responsive on cold
 * start (one round-trip lights up the visible window).
 */
const PAGE_SIZE = 20;

/**
 * Persisted-activity store backing the timeline rail.
 *
 * Three data flows feed `recent`, each routed through a single mutator so
 * the array maintains one invariant: deduped by `id`, sorted by
 * `startedAt` descending. The list is unbounded — growth is whatever the
 * user actually scrolls through.
 *
 * 1. `init()` hydrates the first `PAGE_SIZE` rows from `GET /activities`
 *    via the `activityList` tauri command — the cloud is the source of
 *    truth across restarts.
 * 2. The `savedActivityCreated` tauri event surfaces freshly-tracked
 *    activities as soon as the cloud `POST /activities` succeeds. The
 *    underlying POSTs run as fire-and-forget tokio tasks, so completion
 *    order does not match `startedAt` order — sorting on every apply
 *    keeps the rail consistent regardless of which row's request landed
 *    first.
 * 3. The `savedActivityEnded` tauri event surfaces the closing
 *    `ended_at` once the cloud PATCH succeeds.
 *
 * Both listeners are registered *before* the snapshot fetch so events
 * fired during hydration are not lost; the id-dedupe makes overlap with
 * the snapshot safe, and `applyEnded` is a no-op for ids the snapshot
 * has not yet inserted.
 *
 * A fourth flow loads older history on demand: `loadMore` extends the
 * tail of `recent` with the next page of `GET /activities` and is fired
 * by the rail's bottom-sentinel `IntersectionObserver` as the user
 * scrolls toward the loaded edge. `paginatedOffset` is the server-side
 * cursor for that fetch; it advances **only** by the row count the
 * server returned, never by live prepends, so an event landing mid-fetch
 * can't shift the cursor and cause a row to be missed.
 *
 * On top of the chronological list the service tracks two distinct
 * "current app" concepts that callers must not conflate:
 *
 * - `liveActivity` is whatever the user is *actually* focused on right
 *   now (always `recent[0]`). It feeds the thread→activity link in
 *   `ChatSendRequest.activity_id` — a chat the user starts belongs to
 *   the app they're really in, not the one they happen to be scrolled
 *   to.
 * - `activeApp` is the user's selection in the rail
 *   (`recent[activeIndex]`). The MainSidebar uses it to filter and sort
 *   the threads list. `activeIndex === 0` is the implicit "follow live"
 *   state: a new arrival prepends to `recent` and `activeApp`
 *   automatically tracks the new top. Once the user picks an older row
 *   (`activeIndex > 0`), that becomes an explicit pin to that activity
 *   *by id* — live arrivals shift the index up to keep it pointing at
 *   the same row, and the rail surfaces a "jump to live" affordance so
 *   the user can return to `activeIndex === 0` on demand.
 */
export class ActivityService {
	recent: SavedActivity[] = $state([]);
	activeIndex: number = $state(0);
	hasMore: boolean = $state(true);
	loadingMore: boolean = $state(false);

	liveActivity: SavedActivity | undefined = $derived(this.recent[0]);
	activeApp: SavedActivity | undefined = $derived(this.recent[this.activeIndex]);

	private readonly listeners = new ListenerBag();
	private paginatedOffset = 0;

	async init(): Promise<void> {
		this.listeners.add(
			events.savedActivityCreated.listen((e) => this.applyActivity(e.payload)),
		);
		this.listeners.add(events.savedActivityEnded.listen((e) => this.applyEnded(e.payload)));

		try {
			const snapshot = unwrap(await commands.activityList(PAGE_SIZE, 0));
			// Advance the cursor by the server-confirmed row count *before*
			// applying rows, so a `savedActivityCreated` event landing
			// during the fetch can't end up shifting our pagination view
			// of older history.
			this.paginatedOffset = snapshot.length;
			this.hasMore = snapshot.length === PAGE_SIZE;
			for (const row of snapshot) this.applyActivity(row);
		} catch (error) {
			console.error('Failed to load recent activities:', error);
		}
	}

	async destroy(): Promise<void> {
		this.recent = [];
		this.activeIndex = 0;
		this.hasMore = true;
		this.loadingMore = false;
		this.paginatedOffset = 0;
		await this.listeners.destroy();
	}

	/**
	 * Set the active selection to a specific row. Out-of-range indices
	 * are ignored so callers don't need to guard against a stale render
	 * racing with a list mutation.
	 */
	selectIndex(index: number): void {
		if (index < 0 || index >= this.recent.length) return;
		this.activeIndex = index;
	}

	/**
	 * Advance the active-app selection by one rail position (toward
	 * older activities). Clamped at the oldest loaded row — there is no
	 * wrap-around. Returns `true` when the selection actually moved so
	 * the rail can scroll the new active row into view.
	 */
	selectNext(): boolean {
		if (this.activeIndex >= this.recent.length - 1) return false;
		this.activeIndex += 1;
		return true;
	}

	/**
	 * Inverse of [`selectNext`] — moves the selection toward more recent
	 * activities. Clamped at index 0: the most-recent activity is the
	 * ceiling, so a press past it is a no-op.
	 */
	selectPrevious(): boolean {
		if (this.recent.length === 0) return false;
		if (this.activeIndex === 0) return false;
		this.activeIndex -= 1;
		return true;
	}

	/**
	 * Snap the selection back to the live (most-recent) row. Used by
	 * the rail's "jump to live" affordance and the Home keybinding when
	 * the user has scrolled or clicked away from the top.
	 */
	jumpToLive(): void {
		this.activeIndex = 0;
	}

	/**
	 * Fetch the next page of older activities and merge them into
	 * `recent`. Fired by the rail's bottom-sentinel observer as the
	 * user scrolls toward the loaded edge; safe to call directly if a
	 * future caller wants to pre-warm the rail.
	 *
	 * The `loadingMore` flag prevents re-entry while a fetch is in
	 * flight. Failures clear the flag and leave `hasMore` true so the
	 * next scroll-tick retries; a sticky error would surface as
	 * continuous failed fetches at the edge, which is preferable to a
	 * silent dead-end.
	 */
	async loadMore(): Promise<void> {
		if (this.loadingMore || !this.hasMore) return;
		this.loadingMore = true;
		try {
			const page = unwrap(await commands.activityList(PAGE_SIZE, this.paginatedOffset));
			// Advance the cursor by the row count the server actually
			// returned, *before* routing rows through `applyActivity`.
			// Live prepends arriving mid-fetch are then guaranteed not
			// to move the cursor, and the id-dedup in `applyActivity`
			// makes any overlap between the page and the live stream a
			// no-op.
			this.paginatedOffset += page.length;
			this.hasMore = page.length === PAGE_SIZE;
			for (const row of page) this.applyActivity(row);
		} catch (error) {
			console.error('Failed to load more activities:', error);
		} finally {
			this.loadingMore = false;
		}
	}

	private applyActivity(incoming: SavedActivity): void {
		// `activeIndex === 0` is the implicit "follow live" mode: a new
		// arrival simply becomes the new top and `activeIndex` stays 0.
		// `activeIndex > 0` is an explicit pin to a specific activity
		// *by id* — we re-find that id after the merge so live prepends
		// don't silently shift the user's selection to a neighbouring
		// row.
		const wasFollowingLive = this.activeIndex === 0;
		const previousActiveId = wasFollowingLive ? undefined : this.recent[this.activeIndex]?.id;

		// ISO-8601 timestamps compare lexicographically; no Date parsing needed.
		const merged = [incoming, ...this.recent.filter((a) => a.id !== incoming.id)];
		merged.sort((a, b) => b.startedAt.localeCompare(a.startedAt));
		this.recent = merged;

		if (previousActiveId === undefined) {
			this.activeIndex = 0;
			return;
		}
		// Defensive: id-dedup + no cap means the prior row should always
		// still be present, but if some future caller filters the array
		// externally we fall back to row 0 rather than silently switching
		// the user's selection to an unrelated app.
		const newIndex = this.recent.findIndex((a) => a.id === previousActiveId);
		this.activeIndex = newIndex === -1 ? 0 : newIndex;
	}

	private applyEnded(payload: SavedActivityEnded): void {
		// Whole-array reassignment (not in-place mutation) so reactivity
		// fires regardless of `$state` proxy depth, and so the dedup/sort
		// invariant from `applyActivity` is preserved trivially — `map`
		// keeps order and id-uniqueness untouched. A miss is a no-op so
		// races with the initial snapshot fetch can't drop the update on
		// the floor: the snapshot row will already carry `endedAt`.
		const idx = this.recent.findIndex((a) => a.id === payload.id);
		if (idx === -1) return;
		this.recent = this.recent.map((a, i) =>
			i === idx ? { ...a, endedAt: payload.endedAt } : a,
		);
	}
}

export const ACTIVITY_SERVICE = new InjectionToken<ActivityService>('ActivityService');
