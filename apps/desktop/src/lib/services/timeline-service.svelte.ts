import { applyAccent, clearAccent } from '$lib/services/accent.js';
import { InjectionToken } from '@eurora/shared/context';
import type { TimelineAppEvent } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';
import type { AppearanceService } from '$lib/services/appearance-service.svelte.js';

const RECENT_LIMIT = 5;

export class TimelineService {
	recent: TimelineAppEvent[] = $state([]);
	readonly latest: TimelineAppEvent | null = $derived(
		this.recent.length > 0 ? this.recent[this.recent.length - 1] : null,
	);

	private readonly taurpc: TaurpcService;
	private readonly appearance: AppearanceService;
	private readonly unlisteners: Promise<() => void>[] = [];

	constructor(taurpc: TaurpcService, appearance: AppearanceService) {
		this.taurpc = taurpc;
		this.appearance = appearance;
	}

	init() {
		this.unlisteners.push(
			this.taurpc.timeline.new_app_event.on((event) => {
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
