import type { SubscriptionStatus } from '$lib/services/subscription-service.svelte.js';

declare global {
	namespace App {
		interface PageData {
			subscription?: SubscriptionStatus | null;
		}
	}
}

export {};
