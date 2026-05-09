import { InjectionToken } from '@eurora/shared/context';
import { commands } from '$lib/bindings/specta.bindings.js';
import { unwrap } from '$lib/bindings/result.js';

export class GeneralService {
	autostart = $state(false);

	async init(): Promise<void> {
		const settings = await commands.settingsGetGeneral();
		this.autostart = settings.autostart;
	}

	async setAutostart(enabled: boolean): Promise<void> {
		this.autostart = enabled;
		await this.persist();
	}

	private async persist(): Promise<void> {
		unwrap(
			await commands.settingsSetGeneral({
				autostart: this.autostart,
			}),
		);
	}
}

export const GENERAL_SERVICE = new InjectionToken<GeneralService>('GeneralService');
