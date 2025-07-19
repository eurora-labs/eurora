<script lang="ts">
	import * as Card from '@eurora/ui/components/card/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { CheckCircle, ArrowRight } from '@lucide/svelte';
	import { page } from '$app/stores';
	import { onMount } from 'svelte';

	let sessionId = '';
	let customerEmail = '';

	onMount(() => {
		sessionId = $page.url.searchParams.get('session_id') || '';
		customerEmail = $page.url.searchParams.get('customer_email') || '';
	});

	function goToDashboard() {
		window.location.href = '/dashboard';
	}

	function goToHome() {
		window.location.href = '/';
	}
</script>

<svelte:head>
	<title>Payment Successful - Eurora</title>
	<meta name="description" content="Your payment was successful. Welcome to Eurora!" />
</svelte:head>

<div class="container mx-auto max-w-2xl px-4 py-16">
	<div class="text-center">
		<div class="mb-8 flex justify-center">
			<div class="rounded-full bg-green-100 p-6">
				<CheckCircle class="h-16 w-16 text-green-600" />
			</div>
		</div>

		<h1 class="mb-4 text-4xl font-bold text-gray-900">Payment Successful!</h1>
		<p class="mb-8 text-xl text-gray-600">
			Thank you for subscribing to Eurora. Your payment has been processed successfully.
		</p>

		<Card.Root class="mb-8 text-left">
			<Card.Header>
				<Card.Title>What's Next?</Card.Title>
				<Card.Description
					>Here's what you can expect after your successful payment</Card.Description
				>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div class="flex items-start space-x-3">
						<div
							class="mt-1 flex h-6 w-6 items-center justify-center rounded-full bg-blue-100"
						>
							<span class="text-sm font-medium text-blue-600">1</span>
						</div>
						<div>
							<h3 class="font-medium text-gray-900">Confirmation Email</h3>
							<p class="text-gray-600">
								You'll receive a confirmation email with your subscription details
								and receipt.
							</p>
						</div>
					</div>
					<div class="flex items-start space-x-3">
						<div
							class="mt-1 flex h-6 w-6 items-center justify-center rounded-full bg-blue-100"
						>
							<span class="text-sm font-medium text-blue-600">2</span>
						</div>
						<div>
							<h3 class="font-medium text-gray-900">Account Activation</h3>
							<p class="text-gray-600">
								Your premium features are now active and ready to use.
							</p>
						</div>
					</div>
					<div class="flex items-start space-x-3">
						<div
							class="mt-1 flex h-6 w-6 items-center justify-center rounded-full bg-blue-100"
						>
							<span class="text-sm font-medium text-blue-600">3</span>
						</div>
						<div>
							<h3 class="font-medium text-gray-900">Get Started</h3>
							<p class="text-gray-600">
								Access your dashboard to start using all the premium features.
							</p>
						</div>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		{#if sessionId}
			<div class="mb-6 rounded-lg bg-gray-50 p-4">
				<p class="text-sm text-gray-600">
					<strong>Session ID:</strong>
					{sessionId}
				</p>
				{#if customerEmail}
					<p class="text-sm text-gray-600">
						<strong>Email:</strong>
						{customerEmail}
					</p>
				{/if}
			</div>
		{/if}

		<div class="flex flex-col gap-4 sm:flex-row sm:justify-center">
			<Button onclick={goToDashboard} class="flex items-center gap-2">
				Go to Dashboard
				<ArrowRight class="h-4 w-4" />
			</Button>
			<Button variant="outline" onclick={goToHome}>Back to Home</Button>
		</div>

		<div class="mt-8 text-center">
			<p class="text-sm text-gray-500">
				Need help? Contact our support team at
				<a href="mailto:support@eurora-labs.com" class="text-blue-600 hover:underline">
					support@eurora-labs.com
				</a>
			</p>
		</div>
	</div>
</div>
