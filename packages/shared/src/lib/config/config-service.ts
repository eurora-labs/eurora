import { InjectionToken } from '@eurora/shared/context';

export class ConfigService {
	readonly grpcApiUrl: string;
	readonly restApiUrl: string;

	constructor(grpcApiUrl: string, restApiUrl: string | undefined) {
		this.grpcApiUrl = grpcApiUrl;

		if (restApiUrl) {
			this.restApiUrl = restApiUrl;
		} else {
			this.restApiUrl = grpcApiUrl;
		}
	}
}

export const CONFIG_SERVICE = new InjectionToken<ConfigService>('ConfigService');
