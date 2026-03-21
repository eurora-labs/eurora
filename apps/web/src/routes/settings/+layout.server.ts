import { redirect } from '@sveltejs/kit';
import type { SubscriptionStatus } from '$lib/stores/subscription.js';
import type { LayoutServerLoad } from './$types';

export async function load({ locals, fetch }: Parameters<LayoutServerLoad>[0]) {
	if (!locals.user || !locals.accessToken) {
		redirect(302, '/login?redirect=/settings');
	}

	let subscription: SubscriptionStatus | null = null;

	try {
		const restApiUrl = import.meta.env.VITE_REST_API_URL ?? '';
		const res = await fetch(`${restApiUrl}/payment/subscription`, {
			headers: { Authorization: `Bearer ${locals.accessToken}` },
		});

		if (res.ok) {
			const data: SubscriptionStatus = await res.json();
			subscription = data.subscription_id ? data : null;
		}
	} catch (err) {
		console.error('Failed to fetch subscription server-side:', err);
	}

	return {
		subscription,
	};
}
