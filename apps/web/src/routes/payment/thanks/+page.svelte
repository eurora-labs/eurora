<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { AUTH_SERVICE } from '$lib/services/auth-service.svelte.js';
	import { CONFIG_SERVICE } from '@eurora/shared/config/config-service';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
	import CheckIcon from '@lucide/svelte/icons/circle-check';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import { onDestroy, onMount } from 'svelte';

	const auth = inject(AUTH_SERVICE);
	const REST_API_URL = inject(CONFIG_SERVICE).restApiUrl;

	let status = $state<'loading' | 'complete' | 'failed'>('loading');
	let countdown = $state(5);
	let countdownInterval: ReturnType<typeof setInterval> | undefined;

	const sessionId = page.url.searchParams.get('session_id');

	onDestroy(() => {
		if (countdownInterval) clearInterval(countdownInterval);
	});

	onMount(async () => {
		if (!sessionId) {
			status = 'failed';
			return;
		}

		try {
			await auth.ensureValidToken();

			const res = await fetch(
				`${REST_API_URL}/payment/checkout-status?session_id=${encodeURIComponent(sessionId)}`,
				{
					headers: {
						Authorization: `Bearer ${auth.accessToken}`,
					},
				},
			);

			if (!res.ok) {
				status = 'failed';
				return;
			}

			const data = await res.json();
			status = data.status === 'complete' ? 'complete' : 'failed';

			if (status === 'complete') {
				countdownInterval = setInterval(() => {
					countdown--;
					if (countdown <= 0) {
						clearInterval(countdownInterval);
						goto('/settings');
					}
				}, 1000);
			}
		} catch {
			status = 'failed';
		}
	});
</script>

<div class="flex min-h-[60vh] items-center justify-center px-4">
	{#if status === 'loading'}
		<Card.Root class="w-full max-w-md p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<Loader2Icon class="size-5 animate-spin text-muted-foreground" />
					<Card.Title class="text-2xl">Verifying payment...</Card.Title>
				</div>
				<Card.Description>Please wait while we confirm your payment.</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if status === 'complete'}
		<Card.Root class="w-full max-w-md p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<CheckIcon class="size-5 text-primary" />
					<Card.Title class="text-2xl">Payment Successful!</Card.Title>
				</div>
				<Card.Description>
					Your Pro subscription is now active. You can now enjoy all Pro features in the
					Eurora app. Redirecting in {countdown}s...
				</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-3">
				<Button class="w-full" href="/settings">Go to Settings</Button>
				<Button variant="outline" class="w-full" href="/">Back to Home</Button>
			</Card.Content>
		</Card.Root>
	{:else}
		<Card.Root class="w-full max-w-md p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<AlertCircleIcon class="size-5 text-destructive" />
					<Card.Title class="text-2xl">Payment Not Verified</Card.Title>
				</div>
				<Card.Description>
					We couldn't confirm your payment. If you were charged, please contact support.
				</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-3">
				<Button class="w-full" href="/pricing">Back to Pricing</Button>
				<Button variant="outline" class="w-full" href="/">Back to Home</Button>
			</Card.Content>
		</Card.Root>
	{/if}
</div>
