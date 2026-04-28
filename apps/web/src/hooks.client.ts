import { dev } from '$app/environment';
import { handleErrorWithSentry, init } from '@sentry/sveltekit';
import { env } from '$env/dynamic/public';

const dsn = env.PUBLIC_SENTRY_WEB_DSN;

if (dsn) {
	init({
		dsn,
		environment: env.PUBLIC_SENTRY_ENVIRONMENT || (dev ? 'development' : 'production'),
		release: env.PUBLIC_SENTRY_RELEASE || undefined,
		tracesSampleRate: 0,
		sendDefaultPii: false,
	});
}

export const handleError = handleErrorWithSentry();
