import { unwrap } from '$lib/bindings/result.js';
import {
	commands,
	type TelemetryBootstrap,
	type TelemetryConsent,
} from '$lib/bindings/specta.bindings.js';
import { InjectionToken } from '@eurora/shared/context';
import * as Sentry from '@sentry/svelte';
import { posthog, type PostHogInterface } from 'posthog-js';

const SENSITIVE_HEADER_NAMES = new Set([
	'authorization',
	'cookie',
	'set-cookie',
	'proxy-authorization',
	'x-api-key',
]);

/**
 * Owns the lifecycle of Sentry + PostHog in the desktop app.
 *
 * Reads bootstrap data once at startup (consent + anonymous id + embedded
 * keys), then keeps the SDKs in sync with the user's preferences. When
 * the user toggles a switch in the settings UI, [`refresh`] re-runs the
 * gating logic without a process restart. The Rust client is reapplied
 * through `system.reinit_telemetry` so both surfaces stay coherent.
 *
 * Consent and the anonymous distinct id are stored separately on the
 * Rust side (consent travels with the user via the cloud cache;
 * distinct_id stays local). This service composes the two into the
 * `bootstrap` mirror it serves to the rest of the app.
 *
 * `applySdks` and its callers are async because tearing down a Sentry
 * client involves flushing buffered events; serializing the close ↔ init
 * pair across consecutive opt-out / opt-in keeps us from racing the
 * flush against a fresh `Sentry.init`.
 */
export class TelemetryService {
	bootstrap = $state<TelemetryBootstrap | null>(null);

	private sentryStarted = false;
	private posthogStarted = false;
	private identifiedUserId: string | null = null;

	get consent(): TelemetryConsent | null {
		return this.bootstrap?.consent ?? null;
	}

	get distinctId(): string | null {
		return this.bootstrap?.distinctId ?? null;
	}

	async init(): Promise<void> {
		try {
			this.bootstrap = unwrap(await commands.systemGetTelemetryBootstrap());
		} catch (error) {
			console.error('Failed to fetch telemetry bootstrap:', error);
			return;
		}
		await this.applySdks();
	}

	/**
	 * Re-fetch consent + distinct id from the Rust side and reapply both
	 * SDKs. Called after the settings UI persists a change so
	 * opt-in/opt-out takes effect immediately.
	 */
	async refresh(): Promise<void> {
		if (!this.bootstrap) return;
		try {
			const [consent, local] = await Promise.all([
				commands.settingsGetTelemetryConsent(),
				commands.settingsGetLocalTelemetry(),
			]);
			this.bootstrap = {
				...this.bootstrap,
				consent,
				distinctId: local.distinctId ?? null,
			};
			await this.applySdks();
			await commands.systemReinitTelemetry();
		} catch (error) {
			console.error('Failed to refresh telemetry settings:', error);
		}
	}

	/**
	 * Identify the current authenticated user against PostHog, but only
	 * when the user has explicitly opted into non-anonymous metrics.
	 * Sentry's user is set to the (anonymous) distinct id always — that's
	 * the operator-side join key, not a PII channel.
	 */
	identify(user: { email: string; displayName: string | null; role: string }): void {
		const consent = this.consent;
		if (!consent || !this.posthogStarted) return;
		if (!consent.nonAnonymousMetrics) return;

		const distinctId = this.distinctId;
		if (!distinctId) return;

		this.identifiedUserId = distinctId;
		posthog.identify(distinctId, {
			email: user.email,
			name: user.displayName,
			role: user.role,
		});
	}

	/**
	 * Drop any user identification on logout. The PostHog distinct id
	 * stays the same; only the linked person properties are cleared.
	 */
	reset(): void {
		this.identifiedUserId = null;
		if (this.posthogStarted) {
			posthog.reset();
		}
	}

	/**
	 * Replace the persisted distinct id with a fresh UUID. Used by the
	 * "reset telemetry id" affordance in settings — equivalent to
	 * "forget me" for analytics linkage.
	 */
	async rotateDistinctId(): Promise<void> {
		if (!this.bootstrap) return;
		try {
			const newId = unwrap(await commands.systemRotateTelemetryDistinctId());
			this.bootstrap = { ...this.bootstrap, distinctId: newId };
			this.reset();
			await this.applySdks();
		} catch (error) {
			console.error('Failed to rotate telemetry distinct id:', error);
		}
	}

	capture(event: string, properties?: Record<string, unknown>): void {
		if (!this.consent?.anonymousMetrics || !this.posthogStarted) return;
		posthog.capture(event, properties);
	}

	private async applySdks(): Promise<void> {
		if (!this.bootstrap) return;
		await this.applySentry();
		this.applyPostHog();
	}

	private hasConsented(): boolean {
		return (this.consent?.consentVersion ?? 0) > 0;
	}

	private async applySentry(): Promise<void> {
		if (!this.bootstrap) return;
		const { sentryDsn, channel, release } = this.bootstrap;
		const wantsErrors = this.hasConsented() && this.consent?.anonymousErrors;

		if (!wantsErrors || !sentryDsn) {
			if (this.sentryStarted) {
				// Await so a subsequent re-init doesn't race the flush.
				await Sentry.getClient()?.close(2_000);
				this.sentryStarted = false;
			}
			return;
		}

		if (!this.sentryStarted) {
			try {
				Sentry.init({
					dsn: sentryDsn,
					release: release ?? undefined,
					environment: channel ?? 'dev',
					sendDefaultPii: false,
					attachStacktrace: true,
					tracesSampleRate: 0,
					beforeSend: scrubSentryEvent,
					beforeBreadcrumb: scrubSentryBreadcrumb,
				});
				this.sentryStarted = true;
			} catch (error) {
				// Sentry's init is normally infallible, but a malformed DSN
				// or network init step can throw. Surface it loudly so a
				// "telemetry says enabled, no events landing" report is
				// debuggable from the user's console.
				console.error('Sentry.init failed:', error);
				return;
			}
		}

		const distinctId = this.distinctId;
		Sentry.setUser(distinctId ? { id: distinctId } : null);
	}

	private applyPostHog(): void {
		if (!this.bootstrap) return;
		const consent = this.consent;
		if (!consent) return;
		const { posthogKey, posthogHost } = this.bootstrap;
		const wantsMetrics = this.hasConsented() && consent.anonymousMetrics;

		if (!wantsMetrics || !posthogKey) {
			if (this.posthogStarted) {
				posthog.opt_out_capturing();
			}
			return;
		}

		const distinctId = this.distinctId;
		if (!this.posthogStarted) {
			try {
				posthog.init(posthogKey, {
					api_host: posthogHost ?? 'https://eu.i.posthog.com',
					autocapture: false,
					capture_pageview: false,
					capture_pageleave: false,
					disable_session_recording: true,
					mask_all_text: true,
					mask_all_element_attributes: true,
					respect_dnt: true,
					persistence: 'localStorage',
					bootstrap: distinctId ? { distinctID: distinctId } : undefined,
					loaded: (instance: PostHogInterface) => {
						if (distinctId) {
							instance.register({ distinct_id: distinctId });
						}
					},
				});
				this.posthogStarted = true;
			} catch (error) {
				console.error('posthog.init failed:', error);
				return;
			}
		} else {
			posthog.opt_in_capturing();
		}

		// If the user hasn't opted into non-anonymous metrics but we'd
		// previously identified them, drop the link so subsequent events
		// flow under the anonymous distinct id only.
		if (!consent.nonAnonymousMetrics && this.identifiedUserId) {
			posthog.reset();
			this.identifiedUserId = null;
		}
	}
}

export const TELEMETRY_SERVICE = new InjectionToken<TelemetryService>('TelemetryService');

/**
 * `beforeSend` hook for Sentry events. Sanitizes the request envelope
 * Sentry attaches to thrown errors so OAuth tokens, session cookies,
 * and POST bodies don't leave the machine. The Rust side has its own
 * scrubber for filesystem paths in stack frames; this one handles the
 * web-shaped surfaces that only the JS SDK touches.
 */
function scrubSentryEvent(event: Sentry.ErrorEvent): Sentry.ErrorEvent {
	if (event.request) {
		event.request = scrubRequest(event.request);
	}
	return event;
}

function scrubSentryBreadcrumb(breadcrumb: Sentry.Breadcrumb): Sentry.Breadcrumb {
	const { data } = breadcrumb;
	if (!data) return breadcrumb;

	// `fetch` and `xhr` breadcrumbs from `breadcrumbsIntegration` carry
	// the URL alongside status info. Strip the query string the same way
	// `scrubRequest` strips it from `event.request.url`.
	if (typeof data.url === 'string') {
		data.url = stripQueryString(data.url);
	}
	return breadcrumb;
}

type RequestEventData = NonNullable<Sentry.ErrorEvent['request']>;

function scrubRequest(request: RequestEventData): RequestEventData {
	const scrubbed: RequestEventData = { ...request };
	if (typeof scrubbed.url === 'string') {
		scrubbed.url = stripQueryString(scrubbed.url);
	}
	if (scrubbed.headers) {
		scrubbed.headers = scrubHeaders(scrubbed.headers);
	}
	if (scrubbed.cookies !== undefined) {
		// `cookies` is `Record<string, string>` — we don't enumerate which
		// names are sensitive (they all are, in practice), so collapse the
		// whole map to a single redaction marker.
		scrubbed.cookies = { __redacted__: '[redacted]' };
	}
	if (scrubbed.data !== undefined) {
		// POST/PUT bodies frequently carry credentials, tokens, or PII.
		// Sentry won't capture these unless an integration explicitly
		// attaches them, but if one does we don't want it on the wire.
		scrubbed.data = '[redacted]';
	}
	return scrubbed;
}

function stripQueryString(url: string): string {
	const queryIndex = url.indexOf('?');
	return queryIndex === -1 ? url : url.slice(0, queryIndex);
}

function scrubHeaders(headers: Record<string, string>): Record<string, string> {
	const out: Record<string, string> = {};
	for (const [name, value] of Object.entries(headers)) {
		out[name] = SENSITIVE_HEADER_NAMES.has(name.toLowerCase()) ? '[redacted]' : value;
	}
	return out;
}
