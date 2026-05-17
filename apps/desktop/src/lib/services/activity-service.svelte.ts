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
 * How many rows the rail keeps in memory. The savedActivityCreated
 * handler trims back to it on every prepend, and the initial fetch
 * caps at the same bound.
 */
const RECENT_LIMIT = 20;

/**
 * Persisted-activity store backing the timeline rail.
 *
 * Three data flows feed `recent`, each routed through a single mutator so
 * the array maintains one invariant: deduped by `id`, sorted by
 * `startedAt` descending, capped at `RECENT_LIMIT`.
 *
 * 1. `init()` hydrates from `GET /activities` via the `activityList`
 *    tauri command — the cloud is the source of truth across restarts.
 * 2. The `savedActivityCreated` tauri event surfaces freshly-tracked
 *    activities as soon as the cloud `POST /activities` succeeds. The
 *    underlying POSTs run as fire-and-forget tokio tasks, so completion
 *    order does not match `startedAt` order — sorting on every apply
 *    keeps the rail consistent regardless of which row's request landed
 *    first.
 * 3. The `savedActivityEnded` tauri event surfaces the closing
 *    `ended_at` once the cloud PATCH succeeds. Without it the rail
 *    would keep `endedAt: null` for every row received via (2) and the
 *    duration-based connector height would stay clamped to the minimum
 *    until the next reload re-hydrated from (1).
 *
 * Both listeners are registered *before* the snapshot fetch so events
 * fired during hydration are not lost; the id-dedupe makes overlap with
 * the snapshot safe, and `applyEnded` is a no-op for ids the snapshot
 * has not yet inserted (a later snapshot row will arrive with the right
 * `endedAt` already set).
 */
export class ActivityService {
	recent: SavedActivity[] = $state([]);

	private readonly listeners = new ListenerBag();

	async init(): Promise<void> {
		this.listeners.add(
			events.savedActivityCreated.listen((e) => this.applyActivity(e.payload)),
		);
		this.listeners.add(events.savedActivityEnded.listen((e) => this.applyEnded(e.payload)));

		try {
			const snapshot = unwrap(await commands.activityList(RECENT_LIMIT, 0));
			for (const row of snapshot) this.applyActivity(row);
		} catch (error) {
			console.error('Failed to load recent activities:', error);
		}
	}

	async destroy(): Promise<void> {
		this.recent = [];
		await this.listeners.destroy();
	}

	private applyActivity(incoming: SavedActivity): void {
		// ISO-8601 timestamps compare lexicographically; no Date parsing needed.
		const merged = [incoming, ...this.recent.filter((a) => a.id !== incoming.id)];
		merged.sort((a, b) => b.startedAt.localeCompare(a.startedAt));
		this.recent = merged.slice(0, RECENT_LIMIT);
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
