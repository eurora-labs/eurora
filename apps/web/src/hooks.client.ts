import { dev } from '$app/environment';
import * as Sentry from '@sentry/sveltekit';
import {
	PUBLIC_SENTRY_ENVIRONMENT,
	PUBLIC_SENTRY_RELEASE,
	PUBLIC_SENTRY_WEB_DSN,
} from '$env/static/public';

if (!PUBLIC_SENTRY_WEB_DSN && !dev) {
	console.warn(
		'[sentry] PUBLIC_SENTRY_WEB_DSN is not set; client-side errors will not be reported.',
	);
}

Sentry.init({
	dsn: PUBLIC_SENTRY_WEB_DSN || undefined,
	environment: PUBLIC_SENTRY_ENVIRONMENT || (dev ? 'development' : 'production'),
	release: PUBLIC_SENTRY_RELEASE || undefined,
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

export const handleError = Sentry.handleErrorWithSentry();
