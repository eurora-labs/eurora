import type { User } from '$lib/stores/auth.js';
import type { SubscriptionStatus } from '$lib/stores/subscription.js';

declare global {
	namespace App {
		interface Locals {
			user: User | null;
			accessToken: string | null;
			refreshToken: string | null;
			expiresAt: number | null;
		}

		interface PageData {
			user?: User | null;
			subscription?: SubscriptionStatus | null;
		}
	}
}

export {};
