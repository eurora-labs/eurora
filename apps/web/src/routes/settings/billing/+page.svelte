<script lang="ts">
	import { auth, accessToken } from '$lib/stores/auth.js';
	import {
		subscriptionStore,
		subscription,
		subscriptionError,
	} from '$lib/stores/subscription.js';
	import { CONFIG_SERVICE } from '@eurora/shared/config/config-service';
	import { inject } from '@eurora/shared/context';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
	import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import * as Sentry from '@sentry/sveltekit';

	const { restApiUrl: REST_API_URL } = inject(CONFIG_SERVICE);

	let portalLoading = $state(false);
	let portalError = $state<string | null>(null);

	const planName = $derived($subscription?.price_id ? 'Pro' : 'Free');

	const planPrice = $derived($subscription?.price_id ? '€19.99 / month' : '€0.00 / month');

	const hasPaidPlan = $derived(!!$subscription?.subscription_id);

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

			const res = await fetch(`${REST_API_URL}/payment/portal`, {
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
			Sentry.captureException(err, { tags: { area: 'payment.portal' } });
			portalError = err instanceof Error ? err.message : 'Failed to open billing portal.';
			portalLoading = false;
		}
	}
</script>

<svelte:head>
	<title>Billing - Eurora Labs</title>
</svelte:head>

<div class="space-y-8">
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
			<h3 class="mb-3 text-base font-semibold">Payment &amp; Invoices</h3>
			<p class="mb-4 text-sm text-muted-foreground">
				View invoices, update your payment method, or cancel your subscription through the
				billing portal.
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
					Unlock unlimited queries, premium AI models, faster response times, and priority
					support.
				</p>
				<Button size="sm" href="/pricing">View Plans</Button>
			</Card.Root>
		</div>
	{/if}
</div>
