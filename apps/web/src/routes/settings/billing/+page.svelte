<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth, accessToken, isAuthenticated } from '$lib/stores/auth.js';
	import {
		subscriptionStore,
		subscription,
		subscriptionLoading,
		subscriptionError,
	} from '$lib/stores/subscription.js';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
	import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';

	const PAYMENT_API_URL = import.meta.env.VITE_PAYMENT_API_URL;
	const STRIPE_PRO_PRICE_ID = import.meta.env.VITE_STRIPE_PRO_PRICE_ID;

	let portalLoading = $state(false);
	let portalError = $state<string | null>(null);

	const planName = $derived.by(() => {
		if (!$subscription?.price_id) return 'Free';
		if ($subscription.price_id === STRIPE_PRO_PRICE_ID) return 'Pro';
		return 'Pro'; // Any paid price is treated as a paid plan
	});

	const planPrice = $derived.by(() => {
		if (!$subscription?.price_id) return '$0.00 / month';
		if ($subscription.price_id === STRIPE_PRO_PRICE_ID) return '$20.00 / month';
		return 'Paid plan';
	});

	const hasPaidPlan = $derived(
		!!$subscription?.subscription_id && $subscription?.status === 'active',
	);

	const isCanceling = $derived($subscription?.cancel_at_period_end === true);

	const cancelAtFormatted = $derived.by(() => {
		if (!$subscription?.cancel_at) return null;
		return new Date($subscription.cancel_at * 1000).toLocaleDateString('en-US', {
			month: 'long',
			day: 'numeric',
			year: 'numeric',
		});
	});

	const statusVariant = $derived.by<'default' | 'secondary' | 'destructive' | 'outline'>(() => {
		switch ($subscription?.status) {
			case 'active':
				return 'default';
			case 'past_due':
			case 'unpaid':
				return 'destructive';
			case 'canceled':
				return 'outline';
			default:
				return 'secondary';
		}
	});

	const error = $derived(portalError ?? $subscriptionError);

	async function handleManageBilling() {
		portalLoading = true;
		portalError = null;

		try {
			await auth.ensureValidToken();

			const res = await fetch(`${PAYMENT_API_URL}/payment/portal`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					Authorization: `Bearer ${$accessToken}`,
				},
			});

			if (!res.ok) {
				const body = await res.json().catch(() => null);
				throw new Error(body?.error ?? `Failed to open billing portal (${res.status})`);
			}

			const { url } = await res.json();
			window.location.href = url;
		} catch (err) {
			console.error('Portal error:', err);
			portalError = err instanceof Error ? err.message : 'Failed to open billing portal.';
			portalLoading = false;
		}
	}

	onMount(() => {
		if (!$isAuthenticated) {
			goto('/login?redirect=/settings/billing');
			return;
		}
		// The layout already fetches; this is a no-op if already fetched,
		// or picks up the in-flight request's result.
		subscriptionStore.fetch();
	});
</script>

<svelte:head>
	<title>Billing - Eurora Labs</title>
</svelte:head>

<div class="space-y-8">
	{#if $subscriptionLoading}
		<div class="flex items-center justify-center py-16">
			<Loader2Icon class="h-6 w-6 animate-spin text-muted-foreground" />
		</div>
	{:else}
		{#if error}
			<Card.Root class="border-destructive/50 bg-destructive/5 p-4">
				<div class="flex items-start gap-3">
					<AlertCircleIcon class="mt-0.5 h-4 w-4 text-destructive" />
					<div>
						<p class="text-sm font-medium text-destructive">
							Failed to load billing information
						</p>
						<p class="mt-1 text-sm text-muted-foreground">{error}</p>
						<Button
							variant="outline"
							size="sm"
							class="mt-3"
							onclick={() => subscriptionStore.refresh()}>Retry</Button
						>
					</div>
				</div>
			</Card.Root>
		{/if}

		<div class="flex items-center justify-between py-2">
			<div class="flex items-center gap-3">
				<div>
					<div class="flex items-center gap-2">
						<h3 class="text-2xl font-bold tracking-tight">{planName}</h3>
						{#if hasPaidPlan && $subscription?.status}
							<Badge variant={statusVariant} class="capitalize">
								{isCanceling ? 'Canceling' : $subscription.status}
							</Badge>
						{/if}
					</div>
					<p class="mt-0.5 text-sm text-muted-foreground">{planPrice}</p>
					{#if isCanceling && cancelAtFormatted}
						<p class="mt-1 text-sm text-amber-600">
							Your plan will be canceled on {cancelAtFormatted}
						</p>
					{/if}
				</div>
			</div>
			{#if hasPaidPlan}
				<Button
					variant="outline"
					size="sm"
					onclick={handleManageBilling}
					disabled={portalLoading}
				>
					{#if portalLoading}
						<Loader2Icon class="mr-2 h-4 w-4 animate-spin" />
						Loading...
					{:else}
						Manage Subscription
						<ExternalLinkIcon class="ml-1.5 h-3.5 w-3.5" />
					{/if}
				</Button>
			{:else}
				<Button size="sm" href="/pricing">Upgrade</Button>
			{/if}
		</div>

		{#if hasPaidPlan}
			<div>
				<h3 class="mb-3 text-base font-semibold">Payment & Invoices</h3>
				<p class="mb-4 text-sm text-muted-foreground">
					View invoices, update your payment method, or cancel your subscription through
					the billing portal.
				</p>
				<Button
					variant="outline"
					size="sm"
					onclick={handleManageBilling}
					disabled={portalLoading}
				>
					{#if portalLoading}
						<Loader2Icon class="mr-2 h-4 w-4 animate-spin" />
						Loading...
					{:else}
						Open Billing Portal
						<ExternalLinkIcon class="ml-1.5 h-3.5 w-3.5" />
					{/if}
				</Button>
			</div>
		{:else}
			<div>
				<h3 class="mb-3 text-base font-semibold">Upgrade to Pro</h3>
				<Card.Root class="p-5">
					<p class="mb-1 text-sm font-medium">Get more out of Eurora</p>
					<p class="mb-4 text-sm text-muted-foreground">
						Unlock unlimited queries, premium AI models, faster response times, and
						priority support.
					</p>
					<Button size="sm" href="/pricing">View Plans</Button>
				</Card.Root>
			</div>
		{/if}
	{/if}
</div>
