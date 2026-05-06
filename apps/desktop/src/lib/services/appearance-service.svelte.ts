import { clearAccent } from '$lib/services/accent.js';
import { InjectionToken } from '@eurora/shared/context';
import { setMode } from 'mode-watcher';
import type { AppearanceSettings, Theme } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

const DEFAULT_APPEARANCE: AppearanceSettings = {
	theme: 'system',
	dynamicAccent: true,
};

export class AppearanceService {
	theme = $state<Theme>(DEFAULT_APPEARANCE.theme);
	dynamicAccent = $state<boolean>(DEFAULT_APPEARANCE.dynamicAccent);

	private readonly taurpc: TaurpcService;

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	async init(): Promise<void> {
		const settings = await this.taurpc.settings.get_appearance_settings();
		this.theme = settings.theme;
		this.dynamicAccent = settings.dynamicAccent;
		setMode(this.theme);
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

	private async persist(): Promise<void> {
		await this.taurpc.settings.set_appearance_settings({
			theme: this.theme,
			dynamicAccent: this.dynamicAccent,
		});
	}
}

export const APPEARANCE_SERVICE = new InjectionToken<AppearanceService>('AppearanceService');
