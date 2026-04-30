import { InjectionToken } from '@eurora/shared/context';
import type { AccentColor, TimelineAppEvent } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

const RECENT_LIMIT = 5;

const ACCENT_VARIABLES = [
	'--primary',
	'--primary-foreground',
	'--ring',
	'--accent',
	'--accent-foreground',
	'--sidebar-primary',
	'--sidebar-primary-foreground',
	'--sidebar-ring',
	'--sidebar-accent',
	'--sidebar-accent-foreground',
] as const;

export class TimelineService {
	recent: TimelineAppEvent[] = $state([]);
	readonly latest: TimelineAppEvent | null = $derived(
		this.recent.length > 0 ? this.recent[this.recent.length - 1] : null,
	);

	private readonly taurpc: TaurpcService;
	private readonly unlisteners: Promise<() => void>[] = [];

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	init() {
		this.unlisteners.push(
			this.taurpc.timeline.new_app_event.on((event) => {
				const next = [...this.recent, event];
				this.recent = next.length > RECENT_LIMIT ? next.slice(-RECENT_LIMIT) : next;
				applyAccent(event.accent);
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

function applyAccent(accent: AccentColor | null) {
	if (typeof document === 'undefined') return;
	if (!accent) {
		clearAccent();
		return;
	}
	const root = document.documentElement.style;
	root.setProperty('--primary', accent.hex);
	root.setProperty('--primary-foreground', accent.on_hex);
	root.setProperty('--ring', accent.hex);
	root.setProperty('--accent', accent.hex);
	root.setProperty('--accent-foreground', accent.on_hex);
	root.setProperty('--sidebar-primary', accent.hex);
	root.setProperty('--sidebar-primary-foreground', accent.on_hex);
	root.setProperty('--sidebar-ring', accent.hex);
	root.setProperty('--sidebar-accent', accent.hex);
	root.setProperty('--sidebar-accent-foreground', accent.on_hex);
}

function clearAccent() {
	if (typeof document === 'undefined') return;
	const root = document.documentElement.style;
	for (const variable of ACCENT_VARIABLES) {
		root.removeProperty(variable);
	}
}

export const TIMELINE_SERVICE = new InjectionToken<TimelineService>('TimelineService');
