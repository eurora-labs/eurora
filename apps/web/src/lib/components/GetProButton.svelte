<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth, isAuthenticated, accessToken } from '$lib/stores/auth.js';
	import { Button, type ButtonProps } from '@eurora/ui/components/button/index';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type { Snippet } from 'svelte';

	const REST_API_URL = import.meta.env.VITE_REST_API_URL;
	const STRIPE_PRO_PRICE_ID = import.meta.env.VITE_STRIPE_PRO_PRICE_ID;

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
		if (!$isAuthenticated) {
			goto('/login?redirect=' + encodeURIComponent('/pricing?checkout=true'));
			return;
		}

		loading = true;

		try {
			if (!(await auth.ensureValidToken())) {
				goto('/login?redirect=' + encodeURIComponent('/pricing?checkout=true'));
				return;
			}

			const res = await fetch(`${REST_API_URL}/payment/checkout`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					Authorization: `Bearer ${$accessToken}`,
				},
				body: JSON.stringify({
					price_id: STRIPE_PRO_PRICE_ID,
				}),
			});

			if (!res.ok) {
				const body = await res.json().catch(() => null);
				throw new Error(body?.error ?? `Checkout failed (${res.status})`);
			}

			const { url } = await res.json();
			window.location.href = url;
		} catch (err) {
			console.error('Checkout error:', err);
			toast.error(
				err instanceof Error ? err.message : 'Something went wrong. Please try again.',
			);
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
