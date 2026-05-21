import { ListenerBag } from '$lib/bindings/listeners.js';
import { commands, events, type TimelineAppEvent } from '$lib/bindings/specta.bindings.js';
import { applyAccent, clearAccent } from '$lib/services/accent.js';
import { InjectionToken } from '@eurora/shared/context';
import type { AppearanceService } from '$lib/services/appearance-service.svelte.js';

const RECENT_LIMIT = 100;

export class TimelineService {
	recent: TimelineAppEvent[] = $state([]);
	readonly latest: TimelineAppEvent | null = $derived(
		this.recent.length > 0 ? this.recent[this.recent.length - 1] : null,
	);
	readonly recentDesc: TimelineAppEvent[] = $derived(this.recent.slice().reverse());

	private readonly appearance: AppearanceService;
	private readonly listeners = new ListenerBag();

	constructor(appearance: AppearanceService) {
		this.appearance = appearance;
	}

	init() {
		this.listeners.add(
			events.timelineAppEvent.listen((e) => {
				this.pushEvent(e.payload);
			}),
		);
	}

	/**
	 * Seed `recent` from the backend's current focused-app snapshot.
	 *
	 * Tauri broadcast channels don't replay history, so a webview that
	 * mounts mid-session (the ask / answer overlay windows are the
	 * load-bearing example) would otherwise see no icon until the next
	 * focus change. Calling this once at mount fills the gap with the
	 * activity the timeline collector last observed.
	 *
	 * Idempotent: if `recent` already has entries (the listener already
	 * caught an event) this is a no-op. Skips silently on IPC failure
	 * so a missing/unauthenticated backend can't prevent the overlay
	 * from rendering.
	 */
	async seedFromCurrentActivity(): Promise<void> {
		if (this.recent.length > 0) return;
		try {
			const current = await commands.timelineGetCurrentApp();
			if (current && this.recent.length === 0) {
				this.pushEvent(current);
			}
		} catch (error) {
			console.warn('Failed to seed timeline service from current activity:', error);
		}
	}

	private pushEvent(event: TimelineAppEvent) {
		const next = [...this.recent, event];
		this.recent = next.length > RECENT_LIMIT ? next.slice(-RECENT_LIMIT) : next;
		if (this.appearance.dynamicAccent && event.accent) {
			applyAccent(event.accent);
		} else {
			clearAccent();
		}
	}

	async destroy(): Promise<void> {
		clearAccent();
		this.recent = [];
		await this.listeners.destroy();
	}
}

export const TIMELINE_SERVICE = new InjectionToken<TimelineService>('TimelineService');
