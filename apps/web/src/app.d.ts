import type { SubscriptionStatus } from '$lib/stores/subscription.js';

declare global {
	namespace App {
		interface PageData {
			subscription?: SubscriptionStatus | null;
		}
	}
}

export {};
