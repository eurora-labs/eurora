import { InjectionToken } from '$lib/context.js';

export class ConfigService {
	readonly apiUrl: string;

	constructor(apiUrl: string) {
		this.apiUrl = apiUrl;
	}
}

export const CONFIG_SERVICE = new InjectionToken<ConfigService>('ConfigService');
