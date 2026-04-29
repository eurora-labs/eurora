import { InjectionToken } from '@eurora/shared/context';
import { resolveThemeFromChips, type ThemeName } from '@eurora/ui/themes/index';
import type { ContextChip } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

const THEME_ATTRIBUTE = 'data-theme';

/**
 * Drives the desktop app's active palette by mirroring the currently
 * focused context chip's domain onto `<html data-theme="...">`. The CSS
 * in `@eurora/ui/main.css` reacts to that attribute via per-domain palette
 * files (see `packages/ui/src/styles/themes/`).
 */
export class ThemeService {
	currentTheme = $state<ThemeName>('default');

	private readonly taurpc: TaurpcService;
	private readonly unlisteners: Promise<() => void>[] = [];

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
		// Apply the default theme synchronously so the very first paint
		// already has consistent CSS variables, even before init() resolves.
		this.applyTheme('default');
	}

	async init() {
		this.unlisteners.push(
			this.taurpc.timeline.new_assets_event.on((chips) => {
				this.handleChips(chips);
			}),
		);

		try {
			const chips = await this.taurpc.context_chip.get();
			this.handleChips(chips);
		} catch (error) {
			console.error('[theme-service] failed to seed initial chips:', error);
		}
	}

	destroy() {
		for (const p of this.unlisteners) {
			p.then((unlisten) => unlisten()).catch(() => {});
		}
		this.unlisteners.length = 0;
		this.applyTheme('default');
	}

	private handleChips(chips: ContextChip[] | null | undefined) {
		this.applyTheme(resolveThemeFromChips(chips));
	}

	private applyTheme(theme: ThemeName) {
		if (this.currentTheme === theme) return;
		this.currentTheme = theme;

		if (typeof document === 'undefined') return;
		document.documentElement.setAttribute(THEME_ATTRIBUTE, theme);
	}
}

export const THEME_SERVICE = new InjectionToken<ThemeService>('ThemeService');
