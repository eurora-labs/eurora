import { AUTH_SERVICE, AuthService } from '$lib/services/auth-service.js';
import { CONFIG_SERVICE, ConfigService } from '$lib/services/config-service.js';
import { DOWNLOAD_SERVICE, DownloadService } from '$lib/services/download-service.js';
import { provideAll } from '@eurora/shared/context';

export function initDependencies() {
	const config = new ConfigService({
		grpcApiUrl: import.meta.env.VITE_GRPC_API_URL ?? '',
		restApiUrl: import.meta.env.VITE_REST_API_URL ?? import.meta.env.VITE_GRPC_API_URL ?? '',
		stripeProPriceId: import.meta.env.VITE_STRIPE_PRO_PRICE_ID ?? '',
	});

	provideAll([
		[CONFIG_SERVICE, config],
		[AUTH_SERVICE, new AuthService(config)],
		[DOWNLOAD_SERVICE, new DownloadService(config)],
	]);
}
