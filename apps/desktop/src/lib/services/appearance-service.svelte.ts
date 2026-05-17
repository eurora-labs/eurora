import { unwrap } from '$lib/bindings/result.js';
import {
	commands,
	type DesktopSettings,
	type SharedSettings,
	type ThemePreference,
} from '$lib/bindings/specta.bindings.js';
import { clearAccent } from '$lib/services/accent.js';
import { InjectionToken } from '@eurora/shared/context';
import { setMode } from 'mode-watcher';

/**
 * Bounds and identity value for the accessibility scale sliders. Kept in sync
 * with the matching constants in `crates/common/settings-core/src/desktop.rs`
 * — the backend re-clamps incoming values, so any drift between the two would
 * silently snap the slider on commit.
 */
export const MIN_SCALE = 0.85;
export const MAX_SCALE = 1.5;
export const DEFAULT_SCALE = 1;
export const SCALE_STEP = 0.05;

const UI_SCALE_VAR = '--ui-scale';
const TEXT_SCALE_VAR = '--text-scale';

/**
 * Re-exported alias kept under the historical name. The wire type lives
 * in `settings-core` as `ThemePreference`; the alias avoids churn at the
 * call sites that already speak `Theme`.
 */
export type Theme = ThemePreference;

/**
 * Owns the desktop appearance state. Composes its fields from the cloud
 * `SharedSettings` section (theme, dynamic accent) and `DesktopSettings`
 * section (interface and text scales) — there is no app-side composite
 * type. Reads issue two parallel commands; writes target whichever
 * section actually changed so we don't churn the unrelated file.
 */
export class AppearanceService {
	theme = $state<Theme>('system');
	dynamicAccent = $state<boolean>(true);
	interfaceScale = $state<number>(DEFAULT_SCALE);
	textScale = $state<number>(DEFAULT_SCALE);

	async init(): Promise<void> {
		const [shared, desktop] = await Promise.all([
			commands.settingsGetShared(),
			commands.settingsGetDesktop(),
		]);
		this.theme = shared.theme ?? 'system';
		this.dynamicAccent = shared.dynamicAccent ?? true;
		this.interfaceScale = sanitizeScale(desktop.interfaceScale ?? DEFAULT_SCALE);
		this.textScale = sanitizeScale(desktop.textScale ?? DEFAULT_SCALE);
		setMode(this.theme);
		this.applyScale();
	}

	async setTheme(theme: Theme): Promise<void> {
		this.theme = theme;
		setMode(theme);
		await this.persistShared();
	}

	async setDynamicAccent(enabled: boolean): Promise<void> {
		this.dynamicAccent = enabled;
		if (!enabled) {
			clearAccent();
		}
		await this.persistShared();
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
		await this.persistDesktop();
	}

	async commitTextScale(value: number): Promise<void> {
		this.textScale = sanitizeScale(value);
		this.applyScale();
		await this.persistDesktop();
	}

	async resetScales(): Promise<void> {
		this.interfaceScale = DEFAULT_SCALE;
		this.textScale = DEFAULT_SCALE;
		this.applyScale();
		await this.persistDesktop();
	}

	private applyScale(): void {
		if (typeof document === 'undefined') return;
		const root = document.documentElement;
		root.style.setProperty(UI_SCALE_VAR, String(this.interfaceScale));
		root.style.setProperty(TEXT_SCALE_VAR, String(this.textScale));
	}

	private async persistShared(): Promise<void> {
		const next: SharedSettings = { theme: this.theme, dynamicAccent: this.dynamicAccent };
		unwrap(await commands.settingsSetShared(next));
	}

	/**
	 * Read the current desktop section, patch the scale fields, and write
	 * it back. The desktop section also carries the telemetry consent
	 * block; round-tripping the existing value preserves it untouched, so
	 * scale changes can never accidentally toggle consent.
	 */
	private async persistDesktop(): Promise<void> {
		const current = await commands.settingsGetDesktop();
		const next: DesktopSettings = {
			...current,
			interfaceScale: this.interfaceScale,
			textScale: this.textScale,
		};
		unwrap(await commands.settingsSetDesktop(next));
	}
}

/**
 * Clamp `value` into the supported scale range, replacing any non-finite or
 * missing input with [`DEFAULT_SCALE`]. Accepts `null` because specta-typescript
 * widens Rust `f32` fields to `number | null` (NaN/Infinity round-trip through
 * `serde_json` as JSON null) — the IPC boundary is the only place a null can
 * appear; in-process slider callers always pass a finite number.
 */
function sanitizeScale(value: number | null): number {
	if (value === null || !Number.isFinite(value)) return DEFAULT_SCALE;
	return Math.min(MAX_SCALE, Math.max(MIN_SCALE, value));
}

export const APPEARANCE_SERVICE = new InjectionToken<AppearanceService>('AppearanceService');
