import { dev } from '$app/environment';
import * as Sentry from '@sentry/sveltekit';
import { env } from '$env/dynamic/public';

const dsn = env.PUBLIC_SENTRY_WEB_DSN;

if (dsn) {
	Sentry.init({
		dsn,
		environment: env.PUBLIC_SENTRY_ENVIRONMENT || (dev ? 'development' : 'production'),
		release: env.PUBLIC_SENTRY_RELEASE || undefined,
		tracesSampleRate: 0.05,
		replaysSessionSampleRate: 0,
		replaysOnErrorSampleRate: 1.0,
		sendDefaultPii: false,
		integrations: [
			Sentry.replayIntegration({
				maskAllText: true,
				blockAllMedia: true,
			}),
		],
	});
}

export const handleError = Sentry.handleErrorWithSentry();
