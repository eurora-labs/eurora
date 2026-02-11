<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { auth, accessToken } from '$lib/stores/auth.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import CheckIcon from '@lucide/svelte/icons/circle-check';
	import XCircleIcon from '@lucide/svelte/icons/circle-x';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';

	const PAYMENT_API_URL = import.meta.env.VITE_PAYMENT_API_URL;

	let status = $state<'loading' | 'complete' | 'failed'>('loading');

	const sessionId = page.url.searchParams.get('session_id');

	onMount(async () => {
		if (!sessionId) {
			status = 'failed';
			return;
		}

		try {
			await auth.ensureValidToken();

			const res = await fetch(
				`${PAYMENT_API_URL}/payment/checkout-status?session_id=${encodeURIComponent(sessionId)}`,
				{
					headers: {
						Authorization: `Bearer ${$accessToken}`,
					},
				},
			);

			if (!res.ok) {
				status = 'failed';
				return;
			}

			const data = await res.json();
			status = data.status === 'complete' ? 'complete' : 'failed';
		} catch {
			status = 'failed';
		}
	});
</script>

<div class="flex min-h-[60vh] items-center justify-center px-4">
	{#if status === 'loading'}
		<Card.Root class="max-w-md p-8 text-center">
			<Card.Header>
				<div class="mx-auto mb-4 flex h-16 w-16 items-center justify-center">
					<Loader2Icon class="h-8 w-8 animate-spin text-gray-500" />
				</div>
				<Card.Title class="text-2xl">Verifying payment...</Card.Title>
				<Card.Description>Please wait while we confirm your payment.</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if status === 'complete'}
		<Card.Root class="max-w-md p-8 text-center">
			<Card.Header>
				<div
					class="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-green-100"
				>
					<CheckIcon class="h-8 w-8 text-green-600" />
				</div>
				<Card.Title class="text-2xl">Payment Successful!</Card.Title>
				<Card.Description>
					Your Pro subscription is now active. Thank you for your purchase.
				</Card.Description>
			</Card.Header>
			<Card.Content>
				<div class="mt-6 space-y-3">
					<Button class="w-full" href="/settings">Go to Settings</Button>
					<Button variant="outline" class="w-full" href="/">Back to Home</Button>
				</div>
			</Card.Content>
		</Card.Root>
	{:else}
		<Card.Root class="max-w-md p-8 text-center">
			<Card.Header>
				<div
					class="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-red-100"
				>
					<XCircleIcon class="h-8 w-8 text-red-600" />
				</div>
				<Card.Title class="text-2xl">Payment Not Verified</Card.Title>
				<Card.Description>
					We couldn't confirm your payment. If you were charged, please contact support.
				</Card.Description>
			</Card.Header>
			<Card.Content>
				<div class="mt-6 space-y-3">
					<Button class="w-full" href="/pricing">Back to Pricing</Button>
					<Button variant="outline" class="w-full" href="/">Back to Home</Button>
				</div>
			</Card.Content>
		</Card.Root>
	{/if}
</div>
