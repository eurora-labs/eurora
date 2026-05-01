<script lang="ts">
	import { goto } from '$app/navigation';
	import { AUTH_SERVICE } from '$lib/services/auth-service.svelte.js';
	import { CONFIG_SERVICE } from '@eurora/shared/config/config-service';
	import { inject } from '@eurora/shared/context';
	import { Button, type ButtonProps } from '@eurora/ui/components/button/index';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import * as Sentry from '@sentry/sveltekit';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type { Snippet } from 'svelte';

	const auth = inject(AUTH_SERVICE);
	const { restApiUrl: REST_API_URL } = inject(CONFIG_SERVICE);

	const STRIPE_PRO_PRICE_ID = import.meta.env.VITE_STRIPE_PRO_PRICE_ID ?? '';
	const CHECKOUT_REDIRECT = '/pricing?checkout=true';

	let {
		class: className = '',
		variant = 'default' as ButtonProps['variant'],
		autoTrigger = false,
		children,
		...restProps
	}: ButtonProps & { children?: Snippet; autoTrigger?: boolean } = $props();

	let loading = $state(false);

	onMount(() => {
		if (autoTrigger) handleGetPro();
	});

	async function handleGetPro() {
		if (!auth.isAuthenticated) {
			goto('/login?redirect=' + encodeURIComponent(CHECKOUT_REDIRECT));
			return;
		}

		loading = true;

		try {
			if (!(await auth.ensureValidToken())) {
				goto('/login?redirect=' + encodeURIComponent(CHECKOUT_REDIRECT));
				return;
			}

			const res = await fetch(`${REST_API_URL}/payment/checkout`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					Authorization: `Bearer ${auth.accessToken}`,
				},
				body: JSON.stringify({
					price_id: STRIPE_PRO_PRICE_ID,
				}),
			});

			const body = (await res.json().catch(() => null)) as {
				url?: string;
				error?: string;
			} | null;

			if (!res.ok) {
				throw new Error(body?.error ?? `Checkout failed (${res.status})`);
			}

			if (!body?.url) {
				throw new Error('Checkout response missing redirect URL');
			}
			window.location.href = body.url;
		} catch (err) {
			Sentry.captureException(err, {
				tags: { area: 'payment.checkout' },
				extra: { priceId: STRIPE_PRO_PRICE_ID },
			});
			toast.error(
				err instanceof Error ? err.message : 'Something went wrong. Please try again.',
			);
		} finally {
			loading = false;
		}
	}
</script>

<Button {variant} class={className} onclick={handleGetPro} disabled={loading} {...restProps}>
	{#if loading}
		<Loader2Icon class="mr-2 h-4 w-4 animate-spin" />
		Redirecting...
	{:else}
		{@render children?.()}
	{/if}
</Button>
