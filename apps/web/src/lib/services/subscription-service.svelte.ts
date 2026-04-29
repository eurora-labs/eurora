import { InjectionToken } from '@eurora/shared/context';
import * as Sentry from '@sentry/sveltekit';
import type { AuthService } from '$lib/services/auth-service.svelte.js';
import type { ConfigService } from '@eurora/shared/config/config-service';

export interface SubscriptionStatus {
	subscription_id: string | null;
	status: string | null;
	price_id: string | null;
	cancel_at: number | null;
	cancel_at_period_end: boolean | null;
}

export class SubscriptionService {
	data = $state<SubscriptionStatus | null>(null);
	loading = $state(false);
	error = $state<string | null>(null);
	fetched = $state(false);

	readonly #auth: AuthService;
	readonly #config: ConfigService;

	constructor(auth: AuthService, config: ConfigService) {
		this.#auth = auth;
		this.#config = config;
	}

	async fetch(force = false): Promise<void> {
		if (this.fetched && !force) return;

		this.loading = true;
		this.error = null;

		try {
			await this.#auth.ensureValidToken();
			const token = this.#auth.accessToken;

			const res = await fetch(`${this.#config.restApiUrl}/payment/subscription`, {
				headers: { Authorization: `Bearer ${token}` },
			});

			if (!res.ok) {
				const body = await res.json().catch(() => null);
				throw new Error(body?.error ?? `Failed to load subscription (${res.status})`);
			}

			const data: SubscriptionStatus = await res.json();
			this.data = data.subscription_id ? data : null;
			this.error = null;
			this.fetched = true;
		} catch (err) {
			Sentry.captureException(err, { tags: { area: 'payment.subscription-fetch' } });
			this.error = err instanceof Error ? err.message : 'Failed to load billing information.';
			this.fetched = true;
		} finally {
			this.loading = false;
		}
	}

	refresh(): Promise<void> {
		return this.fetch(true);
	}

	reset(): void {
		this.data = null;
		this.loading = false;
		this.error = null;
		this.fetched = false;
	}
}

export const SUBSCRIPTION_SERVICE = new InjectionToken<SubscriptionService>('SubscriptionService');
