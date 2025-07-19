<script lang="ts">
	import * as Card from '@eurora/ui/components/card/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { XCircle, ArrowLeft, RefreshCw } from '@lucide/svelte';

	function goBack() {
		window.history.back();
	}

	function goToPricing() {
		window.location.href = '/pricing';
	}

	function goToHome() {
		window.location.href = '/';
	}
</script>

<svelte:head>
	<title>Payment Cancelled - Eurora</title>
	<meta name="description" content="Your payment was cancelled. You can try again anytime." />
</svelte:head>

<div class="container mx-auto max-w-2xl px-4 py-16">
	<div class="text-center">
		<div class="mb-8 flex justify-center">
			<div class="rounded-full bg-red-100 p-6">
				<XCircle class="h-16 w-16 text-red-600" />
			</div>
		</div>

		<h1 class="mb-4 text-4xl font-bold text-gray-900">Payment Cancelled</h1>
		<p class="mb-8 text-xl text-gray-600">
			Your payment was cancelled. No charges have been made to your account.
		</p>

		<Card.Root class="mb-8 text-left">
			<Card.Header>
				<Card.Title>What Happened?</Card.Title>
				<Card.Description
					>Your payment process was interrupted or cancelled</Card.Description
				>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div class="flex items-start space-x-3">
						<div
							class="mt-1 flex h-6 w-6 items-center justify-center rounded-full bg-gray-100"
						>
							<span class="text-sm font-medium text-gray-600">•</span>
						</div>
						<div>
							<h3 class="font-medium text-gray-900">No Charges Made</h3>
							<p class="text-gray-600">
								Your payment method was not charged. You can try again anytime.
							</p>
						</div>
					</div>
					<div class="flex items-start space-x-3">
						<div
							class="mt-1 flex h-6 w-6 items-center justify-center rounded-full bg-gray-100"
						>
							<span class="text-sm font-medium text-gray-600">•</span>
						</div>
						<div>
							<h3 class="font-medium text-gray-900">Account Unchanged</h3>
							<p class="text-gray-600">
								Your current plan and features remain the same.
							</p>
						</div>
					</div>
					<div class="flex items-start space-x-3">
						<div
							class="mt-1 flex h-6 w-6 items-center justify-center rounded-full bg-gray-100"
						>
							<span class="text-sm font-medium text-gray-600">•</span>
						</div>
						<div>
							<h3 class="font-medium text-gray-900">Try Again</h3>
							<p class="text-gray-600">
								You can return to our pricing page and try the payment process
								again.
							</p>
						</div>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="mb-8 border-blue-200 bg-blue-50">
			<Card.Content class="pt-6">
				<div class="flex items-start space-x-3">
					<RefreshCw class="mt-1 h-5 w-5 text-blue-600" />
					<div class="text-left">
						<h3 class="font-medium text-blue-900">Need Help?</h3>
						<p class="text-blue-800">
							If you're experiencing issues with payment, our support team is here to
							help. Common issues include:
						</p>
						<ul class="mt-2 list-inside list-disc text-sm text-blue-700">
							<li>Payment method declined</li>
							<li>Browser or network issues</li>
							<li>Questions about pricing or features</li>
						</ul>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<div class="flex flex-col gap-4 sm:flex-row sm:justify-center">
			<Button onclick={goToPricing} class="flex items-center gap-2">
				<RefreshCw class="h-4 w-4" />
				Try Again
			</Button>
			<Button variant="outline" onclick={goBack} class="flex items-center gap-2">
				<ArrowLeft class="h-4 w-4" />
				Go Back
			</Button>
			<Button variant="outline" onclick={goToHome}>Back to Home</Button>
		</div>

		<div class="mt-8 text-center">
			<p class="text-sm text-gray-500">
				Need assistance? Contact our support team at
				<a href="mailto:support@eurora-labs.com" class="text-blue-600 hover:underline">
					support@eurora-labs.com
				</a>
			</p>
		</div>
	</div>
</div>
