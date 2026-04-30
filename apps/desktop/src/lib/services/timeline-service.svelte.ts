import { pickForeground } from '$lib/utils/contrast.js';
import { InjectionToken } from '@eurora/shared/context';
import type { TimelineAppEvent } from '$lib/bindings/bindings.js';
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
				applyAccent(event.color);
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

function applyAccent(color: string | null) {
	if (typeof document === 'undefined') return;
	const foreground = color ? pickForeground(color) : null;
	if (!color || !foreground) {
		clearAccent();
		return;
	}
	const root = document.documentElement.style;
	root.setProperty('--primary', color);
	root.setProperty('--primary-foreground', foreground);
	root.setProperty('--ring', color);
	root.setProperty('--accent', color);
	root.setProperty('--accent-foreground', foreground);
	root.setProperty('--sidebar-primary', color);
	root.setProperty('--sidebar-primary-foreground', foreground);
	root.setProperty('--sidebar-ring', color);
	root.setProperty('--sidebar-accent', color);
	root.setProperty('--sidebar-accent-foreground', foreground);
}

function clearAccent() {
	if (typeof document === 'undefined') return;
	const root = document.documentElement.style;
	for (const variable of ACCENT_VARIABLES) {
		root.removeProperty(variable);
	}
}

export const TIMELINE_SERVICE = new InjectionToken<TimelineService>('TimelineService');
