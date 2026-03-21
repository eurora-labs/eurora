import { InjectionToken } from '@eurora/shared/context';

export interface AppConfig {
	grpcApiUrl: string;
	restApiUrl: string;
	stripeProPriceId: string;
}

export class ConfigService {
	readonly grpcApiUrl: string;
	readonly restApiUrl: string;
	readonly stripeProPriceId: string;

	constructor(config: AppConfig) {
		this.grpcApiUrl = config.grpcApiUrl;
		this.restApiUrl = config.restApiUrl;
		this.stripeProPriceId = config.stripeProPriceId;
	}
}

export const CONFIG_SERVICE = new InjectionToken<ConfigService>('ConfigService');
