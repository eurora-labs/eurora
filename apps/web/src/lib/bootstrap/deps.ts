import { AUTH_SERVICE, AuthService } from '$lib/services/auth-service.js';
import { DOWNLOAD_SERVICE, DownloadService } from '$lib/services/download-service.js';
import { CONFIG_SERVICE, ConfigService } from '@eurora/shared/config/config-service';
import { provideAll } from '@eurora/shared/context';

export function initDependencies() {
	const config = new ConfigService(
		import.meta.env.VITE_GRPC_API_URL,
		import.meta.env.VITE_REST_API_URL ?? import.meta.env.VITE_GRPC_API_URL,
	);

	provideAll([
		[CONFIG_SERVICE, config],
		[AUTH_SERVICE, new AuthService(config)],
		[DOWNLOAD_SERVICE, new DownloadService(config)],
	]);
}
