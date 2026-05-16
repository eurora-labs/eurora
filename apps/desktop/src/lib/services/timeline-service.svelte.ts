import { ListenerBag } from '$lib/bindings/listeners.js';
import { events, type TimelineAppEvent } from '$lib/bindings/specta.bindings.js';
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
				const event = e.payload;
				const next = [...this.recent, event];
				this.recent = next.length > RECENT_LIMIT ? next.slice(-RECENT_LIMIT) : next;
				if (this.appearance.dynamicAccent && event.accent) {
					applyAccent(event.accent);
				} else {
					clearAccent();
				}
			}),
		);
	}

	async destroy(): Promise<void> {
		clearAccent();
		this.recent = [];
		await this.listeners.destroy();
	}
}

export const TIMELINE_SERVICE = new InjectionToken<TimelineService>('TimelineService');
