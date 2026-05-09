import { applyAccent, clearAccent } from '$lib/services/accent.js';
import { events, type TimelineAppEvent } from '$lib/bindings/specta.bindings.js';
import { InjectionToken } from '@eurora/shared/context';
import type { AppearanceService } from '$lib/services/appearance-service.svelte.js';

const RECENT_LIMIT = 5;

export class TimelineService {
	recent: TimelineAppEvent[] = $state([]);
	readonly latest: TimelineAppEvent | null = $derived(
		this.recent.length > 0 ? this.recent[this.recent.length - 1] : null,
	);

	private readonly appearance: AppearanceService;
	private readonly unlisteners: Promise<() => void>[] = [];

	constructor(appearance: AppearanceService) {
		this.appearance = appearance;
	}

	init() {
		this.unlisteners.push(
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

	destroy() {
		clearAccent();
		this.recent = [];
		for (const p of this.unlisteners) {
			p.then((unlisten) => unlisten());
		}
		this.unlisteners.length = 0;
	}
}

export const TIMELINE_SERVICE = new InjectionToken<TimelineService>('TimelineService');
