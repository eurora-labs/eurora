import { InjectionToken } from '@eurora/shared/context';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

export class GeneralService {
	autostart = $state(false);

	private readonly taurpc: TaurpcService;

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	async init(): Promise<void> {
		const settings = await this.taurpc.settings.get_general_settings();
		this.autostart = settings.autostart;
	}

	async setAutostart(enabled: boolean): Promise<void> {
		this.autostart = enabled;
		await this.persist();
	}

	private async persist(): Promise<void> {
		await this.taurpc.settings.set_general_settings({
			autostart: this.autostart,
		});
	}
}

export const GENERAL_SERVICE = new InjectionToken<GeneralService>('GeneralService');
