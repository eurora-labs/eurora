import { clearAccent } from '$lib/services/accent.js';
import { InjectionToken } from '@eurora/shared/context';
import { setMode } from 'mode-watcher';
import type { AppearanceSettings, Theme } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

const DEFAULT_APPEARANCE: AppearanceSettings = {
	theme: 'system',
	dynamicAccent: true,
	interfaceScale: 1,
	textScale: 1,
};

/**
 * Bounds and identity value for the accessibility scale sliders. Kept in sync
 * with the matching constants in `crates/app/euro-settings/src/appearance_settings.rs`
 * — the backend re-clamps incoming values, so any drift between the two would
 * silently snap the slider on commit.
 */
export const MIN_SCALE = 0.85;
export const MAX_SCALE = 1.5;
export const DEFAULT_SCALE = 1;
export const SCALE_STEP = 0.05;

const UI_SCALE_VAR = '--ui-scale';
const TEXT_SCALE_VAR = '--text-scale';

export class AppearanceService {
	theme = $state<Theme>(DEFAULT_APPEARANCE.theme);
	dynamicAccent = $state<boolean>(DEFAULT_APPEARANCE.dynamicAccent);
	interfaceScale = $state<number>(DEFAULT_APPEARANCE.interfaceScale);
	textScale = $state<number>(DEFAULT_APPEARANCE.textScale);

	private readonly taurpc: TaurpcService;

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	async init(): Promise<void> {
		const settings = await this.taurpc.settings.get_appearance_settings();
		this.theme = settings.theme;
		this.dynamicAccent = settings.dynamicAccent;
		this.interfaceScale = sanitizeScale(settings.interfaceScale);
		this.textScale = sanitizeScale(settings.textScale);
		setMode(this.theme);
		this.applyScale();
	}

	async setTheme(theme: Theme): Promise<void> {
		this.theme = theme;
		setMode(theme);
		await this.persist();
	}

	async setDynamicAccent(enabled: boolean): Promise<void> {
		this.dynamicAccent = enabled;
		if (!enabled) {
			clearAccent();
		}
		await this.persist();
	}

	/**
	 * Update the interface-scale state and CSS variables without touching the
	 * backend. Wired to the slider's `onValueChange` so dragging produces
	 * instant visual feedback without thrashing the settings file.
	 */
	previewInterfaceScale(value: number): void {
		this.interfaceScale = sanitizeScale(value);
		this.applyScale();
	}

	previewTextScale(value: number): void {
		this.textScale = sanitizeScale(value);
		this.applyScale();
	}

	/**
	 * Persist the current scales after the user releases the slider thumb.
	 * Wired to the slider's `onValueCommit`. The argument is taken as the
	 * authoritative final value (covers the case where the slider reports a
	 * commit without first emitting a matching change).
	 */
	async commitInterfaceScale(value: number): Promise<void> {
		this.interfaceScale = sanitizeScale(value);
		this.applyScale();
		await this.persist();
	}

	async commitTextScale(value: number): Promise<void> {
		this.textScale = sanitizeScale(value);
		this.applyScale();
		await this.persist();
	}

	async resetScales(): Promise<void> {
		this.interfaceScale = DEFAULT_SCALE;
		this.textScale = DEFAULT_SCALE;
		this.applyScale();
		await this.persist();
	}

	private applyScale(): void {
		if (typeof document === 'undefined') return;
		const root = document.documentElement;
		root.style.setProperty(UI_SCALE_VAR, String(this.interfaceScale));
		root.style.setProperty(TEXT_SCALE_VAR, String(this.textScale));
	}

	private async persist(): Promise<void> {
		await this.taurpc.settings.set_appearance_settings({
			theme: this.theme,
			dynamicAccent: this.dynamicAccent,
			interfaceScale: this.interfaceScale,
			textScale: this.textScale,
		});
	}
}

function sanitizeScale(value: number): number {
	if (!Number.isFinite(value)) return DEFAULT_SCALE;
	return Math.min(MAX_SCALE, Math.max(MIN_SCALE, value));
}

export const APPEARANCE_SERVICE = new InjectionToken<AppearanceService>('AppearanceService');
