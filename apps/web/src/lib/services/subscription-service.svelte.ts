import { ApiClient } from '$lib/api/client.js';
import { InjectionToken } from '@eurora/shared/context';
import * as Sentry from '@sentry/sveltekit';
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

	readonly #api: ApiClient;

	constructor(config: ConfigService) {
		this.#api = new ApiClient(config);
	}

	async fetch(force = false): Promise<void> {
		if (this.fetched && !force) return;

		this.loading = true;
		this.error = null;

		try {
			const data = await this.#api.fetch<SubscriptionStatus>('/payment/subscription');
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
