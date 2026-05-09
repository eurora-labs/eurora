import { InjectionToken } from '@eurora/shared/context';
import * as Sentry from '@sentry/svelte';
import { posthog, type PostHogInterface } from 'posthog-js';
import {
	commands,
	type TelemetryBootstrap,
	type TelemetrySettings,
} from '$lib/bindings/specta.bindings.js';
import { unwrap } from '$lib/bindings/result.js';

/**
 * Owns the lifecycle of Sentry + PostHog in the desktop app.
 *
 * Reads bootstrap data once at startup (consent + embedded keys), then
 * keeps the SDKs in sync with the user's preferences. When the user
 * toggles a switch in the settings UI, [`apply`] re-runs the gating
 * logic without a process restart. The Rust client is reapplied through
 * `system.reinit_telemetry` so both surfaces stay coherent.
 */
export class TelemetryService {
	settings = $state<TelemetrySettings | null>(null);

	private bootstrap: TelemetryBootstrap | null = null;
	private sentryStarted = false;
	private posthogStarted = false;
	private identifiedUserId: string | null = null;

	async init(): Promise<void> {
		try {
			this.bootstrap = unwrap(await commands.systemGetTelemetryBootstrap());
		} catch (error) {
			console.error('Failed to fetch telemetry bootstrap:', error);
			return;
		}
		this.settings = this.bootstrap.settings;
		this.applySdks();
	}

	/**
	 * Re-fetch settings from the Rust side and reapply both SDKs. Called
	 * after the settings UI persists a change so opt-in/opt-out takes
	 * effect immediately.
	 */
	async refresh(): Promise<void> {
		if (!this.bootstrap) return;
		try {
			const next = await commands.settingsGetTelemetry();
			this.bootstrap = { ...this.bootstrap, settings: next };
			this.settings = next;
			this.applySdks();
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
		if (!this.settings || !this.posthogStarted) return;
		if (!this.settings.nonAnonymousMetrics) return;

		const distinctId = this.settings.distinctId;
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
			if (this.settings) {
				this.settings = { ...this.settings, distinctId: newId };
				this.bootstrap = { ...this.bootstrap, settings: this.settings };
			}
			this.reset();
			this.applySdks();
		} catch (error) {
			console.error('Failed to rotate telemetry distinct id:', error);
		}
	}

	capture(event: string, properties?: Record<string, unknown>): void {
		if (!this.settings?.anonymousMetrics || !this.posthogStarted) return;
		posthog.capture(event, properties);
	}

	private applySdks(): void {
		if (!this.bootstrap || !this.settings) return;

		this.applySentry();
		this.applyPostHog();
	}

	private hasConsented(): boolean {
		return (this.settings?.consentVersion ?? 0) > 0;
	}

	private applySentry(): void {
		if (!this.bootstrap) return;
		const { sentryDsn, channel, release } = this.bootstrap;
		const wantsErrors = this.hasConsented() && this.settings?.anonymousErrors;

		if (!wantsErrors || !sentryDsn) {
			if (this.sentryStarted) {
				Sentry.getClient()?.close(2_000);
				this.sentryStarted = false;
			}
			return;
		}

		if (!this.sentryStarted) {
			Sentry.init({
				dsn: sentryDsn,
				release: release || undefined,
				environment: channel || 'dev',
				sendDefaultPii: false,
				attachStacktrace: true,
				tracesSampleRate: 0,
				replaysSessionSampleRate: 0,
				replaysOnErrorSampleRate: 0,
				beforeSend(event) {
					// Drop the URL query string — it can carry one-time
					// auth tokens through OAuth callbacks.
					if (event.request?.url) {
						event.request.url = event.request.url.split('?')[0];
					}
					return event;
				},
			});
			this.sentryStarted = true;
		}

		const distinctId = this.settings?.distinctId ?? null;
		Sentry.setUser(distinctId ? { id: distinctId } : null);
	}

	private applyPostHog(): void {
		if (!this.bootstrap || !this.settings) return;
		const { posthogKey, posthogHost } = this.bootstrap;
		const wantsMetrics = this.hasConsented() && this.settings.anonymousMetrics;

		if (!wantsMetrics || !posthogKey) {
			if (this.posthogStarted) {
				posthog.opt_out_capturing();
			}
			return;
		}

		if (!this.posthogStarted) {
			posthog.init(posthogKey, {
				api_host: posthogHost || 'https://eu.i.posthog.com',
				autocapture: false,
				capture_pageview: false,
				capture_pageleave: false,
				disable_session_recording: true,
				mask_all_text: true,
				mask_all_element_attributes: true,
				respect_dnt: true,
				persistence: 'localStorage',
				bootstrap: this.settings.distinctId
					? { distinctID: this.settings.distinctId }
					: undefined,
				loaded: (instance: PostHogInterface) => {
					if (this.settings?.distinctId) {
						instance.register({ distinct_id: this.settings.distinctId });
					}
				},
			});
			this.posthogStarted = true;
		} else {
			posthog.opt_in_capturing();
		}

		// If the user hasn't opted into non-anonymous metrics but we'd
		// previously identified them, drop the link so subsequent events
		// flow under the anonymous distinct id only.
		if (!this.settings.nonAnonymousMetrics && this.identifiedUserId) {
			posthog.reset();
			this.identifiedUserId = null;
		}
	}
}

export const TELEMETRY_SERVICE = new InjectionToken<TelemetryService>('TelemetryService');
