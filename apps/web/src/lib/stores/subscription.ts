import { auth, accessToken } from '$lib/stores/auth.js';
import { writable, derived, get } from 'svelte/store';

const REST_API_URL = import.meta.env.VITE_REST_API_URL;

export interface SubscriptionStatus {
	subscription_id: string | null;
	status: string | null;
	price_id: string | null;
	cancel_at: number | null;
	cancel_at_period_end: boolean | null;
}

interface SubscriptionState {
	data: SubscriptionStatus | null;
	loading: boolean;
	error: string | null;
	fetched: boolean;
}

const store = writable<SubscriptionState>({
	data: null,
	loading: false,
	error: null,
	fetched: false,
});

export const subscriptionStore = {
	subscribe: store.subscribe,

	async fetch(force = false) {
		const current = get(store);
		if (current.fetched && !force) return;

		store.update((s) => ({ ...s, loading: true, error: null }));

		try {
			await auth.ensureValidToken();
			const token = get(accessToken);

			const res = await fetch(`${REST_API_URL}/payment/subscription`, {
				headers: { Authorization: `Bearer ${token}` },
			});

			if (!res.ok) {
				const body = await res.json().catch(() => null);
				throw new Error(body?.error ?? `Failed to load subscription (${res.status})`);
			}

			const data: SubscriptionStatus = await res.json();
			store.set({
				data: data.subscription_id ? data : null,
				loading: false,
				error: null,
				fetched: true,
			});
		} catch (err) {
			console.error('Failed to fetch subscription:', err);
			store.update((s) => ({
				...s,
				loading: false,
				error: err instanceof Error ? err.message : 'Failed to load billing information.',
				fetched: true,
			}));
		}
	},

	async refresh() {
		return await this.fetch(true);
	},

	reset() {
		store.set({ data: null, loading: false, error: null, fetched: false });
	},
};

export const subscription = derived(store, ($s) => $s.data);
export const subscriptionLoading = derived(store, ($s) => $s.loading);
export const subscriptionError = derived(store, ($s) => $s.error);
