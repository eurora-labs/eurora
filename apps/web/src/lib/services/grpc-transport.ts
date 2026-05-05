import { createGrpcWebTransport } from '@connectrpc/connect-web';
import type { AuthService } from '$lib/services/auth-service.svelte.js';
import type { Interceptor, Transport } from '@connectrpc/connect';
import type { ConfigService } from '@eurora/shared/config/config-service';

export function createAuthedTransport(config: ConfigService, auth: AuthService): Transport {
	return createGrpcWebTransport({
		baseUrl: config.grpcApiUrl,
		useBinaryFormat: true,
		interceptors: [authInterceptor(auth)],
	});
}

function authInterceptor(auth: AuthService): Interceptor {
	return (next) => async (req) => {
		await auth.ensureValidToken();
		const token = auth.accessToken;
		if (token) {
			req.header.set('authorization', `Bearer ${token}`);
		}
		return await next(req);
	};
}
