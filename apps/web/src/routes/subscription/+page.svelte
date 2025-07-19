<script lang="ts">
	import * as Card from '@eurora/ui/components/card/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { CreditCard, Calendar, AlertCircle, CheckCircle, XCircle } from '@lucide/svelte';
	import { getStripeService } from '$lib/stripe/service.js';
	import { onMount } from 'svelte';

	const stripeService = getStripeService();

	// Mock subscription data - in a real app, this would come from your backend
	let subscription = {
		id: 'sub_1234567890',
		status: 'active',
		plan: {
			name: 'Pro',
			amount: 1200, // $12.00 in cents
			currency: 'usd',
			interval: 'month',
		},
		current_period_start: '2024-01-01T00:00:00Z',
		current_period_end: '2024-02-01T00:00:00Z',
		cancel_at_period_end: false,
		customer: {
			email: 'user@example.com',
		},
	};

	let loading = false;

	function getStatusBadge(status: string): {
		variant: 'default' | 'destructive' | 'secondary' | 'outline';
		icon: any;
		text: string;
	} {
		switch (status) {
			case 'active':
				return { variant: 'default', icon: CheckCircle, text: 'Active' };
			case 'canceled':
				return { variant: 'destructive', icon: XCircle, text: 'Canceled' };
			case 'past_due':
				return { variant: 'destructive', icon: AlertCircle, text: 'Past Due' };
			case 'trialing':
				return { variant: 'secondary', icon: Calendar, text: 'Trial' };
			default:
				return { variant: 'secondary', icon: AlertCircle, text: status };
		}
	}

	function formatDate(dateString: string) {
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'long',
			day: 'numeric',
		});
	}

	async function cancelSubscription() {
		if (
			!confirm(
				'Are you sure you want to cancel your subscription? You will continue to have access until the end of your current billing period.',
			)
		) {
			return;
		}

		loading = true;
		try {
			// In a real app, you would call your backend API to cancel the subscription
			console.log('Canceling subscription:', subscription.id);

			// Mock the cancellation
			subscription.cancel_at_period_end = true;
			alert(
				'Your subscription has been scheduled for cancellation at the end of the current billing period.',
			);
		} catch (error) {
			console.error('Error canceling subscription:', error);
			alert(
				'There was an error canceling your subscription. Please try again or contact support.',
			);
		} finally {
			loading = false;
		}
	}

	async function reactivateSubscription() {
		loading = true;
		try {
			// In a real app, you would call your backend API to reactivate the subscription
			console.log('Reactivating subscription:', subscription.id);

			// Mock the reactivation
			subscription.cancel_at_period_end = false;
			alert('Your subscription has been reactivated and will continue automatically.');
		} catch (error) {
			console.error('Error reactivating subscription:', error);
			alert(
				'There was an error reactivating your subscription. Please try again or contact support.',
			);
		} finally {
			loading = false;
		}
	}

	function updatePaymentMethod() {
		// In a real app, you would redirect to a payment method update flow
		alert(
			'Payment method update functionality would be implemented here using Stripe Customer Portal or custom forms.',
		);
	}

	function downloadInvoices() {
		// In a real app, you would fetch and download invoices
		alert('Invoice download functionality would be implemented here.');
	}

	onMount(() => {
		// In a real app, you would fetch the subscription data from your backend
		console.log('Loading subscription data...');
	});

	$: statusBadge = getStatusBadge(subscription.status);
</script>

<svelte:head>
	<title>Subscription Management - Eurora</title>
	<meta
		name="description"
		content="Manage your Eurora subscription, billing, and payment methods."
	/>
</svelte:head>

<div class="container mx-auto max-w-4xl px-4 py-8">
	<div class="mb-8">
		<h1 class="mb-2 text-3xl font-bold">Subscription Management</h1>
		<p class="text-gray-600">Manage your subscription, billing, and payment methods</p>
	</div>

	<div class="grid gap-6 md:grid-cols-2">
		<!-- Current Plan -->
		<Card.Root>
			<Card.Header>
				<div class="flex items-center justify-between">
					<Card.Title>Current Plan</Card.Title>
					<Badge variant={statusBadge.variant} class="flex items-center gap-1">
						<svelte:component this={statusBadge.icon} class="h-3 w-3" />
						{statusBadge.text}
					</Badge>
				</div>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div>
						<h3 class="text-lg font-semibold">{subscription.plan.name}</h3>
						<p class="text-2xl font-bold">
							{stripeService.formatPrice(
								subscription.plan.amount / 100,
								subscription.plan.currency,
							)}
							<span class="text-sm font-normal text-gray-600"
								>/{subscription.plan.interval}</span
							>
						</p>
					</div>

					<div class="space-y-2">
						<div class="flex justify-between text-sm">
							<span class="text-gray-600">Customer Email:</span>
							<span>{subscription.customer.email}</span>
						</div>
						<div class="flex justify-between text-sm">
							<span class="text-gray-600">Subscription ID:</span>
							<span class="font-mono text-xs">{subscription.id}</span>
						</div>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<!-- Billing Information -->
		<Card.Root>
			<Card.Header>
				<Card.Title class="flex items-center gap-2">
					<Calendar class="h-5 w-5" />
					Billing Information
				</Card.Title>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div class="space-y-2">
						<div class="flex justify-between text-sm">
							<span class="text-gray-600">Current Period:</span>
							<span>{formatDate(subscription.current_period_start)}</span>
						</div>
						<div class="flex justify-between text-sm">
							<span class="text-gray-600">Next Billing Date:</span>
							<span>{formatDate(subscription.current_period_end)}</span>
						</div>
						{#if subscription.cancel_at_period_end}
							<div class="rounded-md bg-yellow-50 p-3">
								<p class="text-sm text-yellow-800">
									<AlertCircle class="mr-1 inline h-4 w-4" />
									Your subscription will be canceled on {formatDate(
										subscription.current_period_end,
									)}
								</p>
							</div>
						{/if}
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<!-- Payment Method -->
		<Card.Root>
			<Card.Header>
				<Card.Title class="flex items-center gap-2">
					<CreditCard class="h-5 w-5" />
					Payment Method
				</Card.Title>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div class="flex items-center space-x-3">
						<div class="rounded-md bg-gray-100 p-2">
							<CreditCard class="h-6 w-6 text-gray-600" />
						</div>
						<div>
							<p class="font-medium">•••• •••• •••• 4242</p>
							<p class="text-sm text-gray-600">Expires 12/25</p>
						</div>
					</div>
					<Button variant="outline" onclick={updatePaymentMethod} class="w-full">
						Update Payment Method
					</Button>
				</div>
			</Card.Content>
		</Card.Root>

		<!-- Actions -->
		<Card.Root>
			<Card.Header>
				<Card.Title>Subscription Actions</Card.Title>
			</Card.Header>
			<Card.Content>
				<div class="space-y-3">
					{#if subscription.cancel_at_period_end}
						<Button onclick={reactivateSubscription} disabled={loading} class="w-full">
							{loading ? 'Processing...' : 'Reactivate Subscription'}
						</Button>
					{:else}
						<Button
							variant="destructive"
							onclick={cancelSubscription}
							disabled={loading}
							class="w-full"
						>
							{loading ? 'Processing...' : 'Cancel Subscription'}
						</Button>
					{/if}

					<Button variant="outline" onclick={downloadInvoices} class="w-full">
						Download Invoices
					</Button>

					<Button
						variant="outline"
						onclick={() => (window.location.href = '/pricing')}
						class="w-full"
					>
						Change Plan
					</Button>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

	<!-- Support -->
	<Card.Root class="mt-6">
		<Card.Header>
			<Card.Title>Need Help?</Card.Title>
		</Card.Header>
		<Card.Content>
			<p class="mb-4 text-gray-600">
				If you have questions about your subscription or need assistance, our support team
				is here to help.
			</p>
			<div class="flex flex-col gap-2 sm:flex-row">
				<Button
					variant="outline"
					onclick={() => (window.location.href = 'mailto:support@eurora-labs.com')}
				>
					Contact Support
				</Button>
				<Button variant="outline" onclick={() => (window.location.href = '/help')}>
					View Help Center
				</Button>
			</div>
		</Card.Content>
	</Card.Root>
</div>
