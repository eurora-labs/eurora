import * as Sentry from '@sentry/sveltekit';
import { env } from '$env/dynamic/private';

const dsn = env.SENTRY_DSN;

if (dsn) {
	Sentry.init({
		dsn,
		environment: env.SENTRY_ENVIRONMENT || 'production',
		release: env.SENTRY_RELEASE || undefined,
		tracesSampleRate: 0.05,
		enableLogs: true,
		sendDefaultPii: false,
	});
}
