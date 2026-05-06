import type { AccentColor } from '$lib/bindings/bindings.js';

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

export function applyAccent(accent: AccentColor): void {
	if (typeof document === 'undefined') return;
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

export function clearAccent(): void {
	if (typeof document === 'undefined') return;
	const root = document.documentElement.style;
	for (const variable of ACCENT_VARIABLES) {
		root.removeProperty(variable);
	}
}
