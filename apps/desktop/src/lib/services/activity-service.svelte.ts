import { ListenerBag } from '$lib/bindings/listeners.js';
import { unwrap } from '$lib/bindings/result.js';
import {
	commands,
	events,
	type SavedActivity,
} from '$lib/bindings/specta.bindings.js';
import { InjectionToken } from '@eurora/shared/context';

/**
 * How many rows the rail keeps in memory. Matches the activity service's
 * `DEFAULT_LIST_LIMIT` (20) so the initial fetch fills the bound without
 * pagination, and the savedActivityCreated handler trims back to it on
 * every prepend.
 */
const RECENT_LIMIT = 20;

/**
 * Persisted-activity store backing the timeline rail.
 *
 * Two data flows feed `recent`, both routed through `applyActivity` so the
 * array maintains a single invariant: deduped by `id`, sorted by
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
 *
 * The listener is registered *before* the snapshot fetch so an event
 * fired during hydration is not lost; the id-dedupe makes overlap with
 * the snapshot safe.
 */
export class ActivityService {
	recent: SavedActivity[] = $state([]);

	private readonly listeners = new ListenerBag();

	async init(): Promise<void> {
		this.listeners.add(
			events.savedActivityCreated.listen((e) => this.applyActivity(e.payload)),
		);

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
		const merged = [
			incoming,
			...this.recent.filter((a) => a.id !== incoming.id),
		];
		merged.sort((a, b) => b.startedAt.localeCompare(a.startedAt));
		this.recent = merged.slice(0, RECENT_LIMIT);
	}
}

export const ACTIVITY_SERVICE = new InjectionToken<ActivityService>('ActivityService');
