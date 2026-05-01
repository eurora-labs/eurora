<script lang="ts">
	import { page } from '$app/state';
	import { AUTH_SERVICE } from '$lib/services/auth-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import CircleX from '@lucide/svelte/icons/circle-x';
	import * as Sentry from '@sentry/sveltekit';
	import { onMount } from 'svelte';

	const auth = inject(AUTH_SERVICE);

	let status: 'verifying' | 'success' | 'error' = $state('verifying');
	let errorMessage = $state('');

	onMount(async () => {
		const token = page.url.searchParams.get('token');
		if (!token) {
			status = 'error';
			errorMessage = 'No verification token provided.';
			return;
		}

		try {
			await auth.verifyEmail(token);
			status = 'success';
		} catch (err) {
			Sentry.captureException(err, { tags: { area: 'auth.verify-email' } });
			status = 'error';
			errorMessage =
				err instanceof Error ? err.message : 'Invalid or expired verification token.';
		}
	});
</script>

<svelte:head>
	<title>Verify Email - Eurora Labs</title>
</svelte:head>

<div class="flex min-h-screen items-center justify-center px-4">
	<div class="w-full max-w-md space-y-8">
		<Card.Root class="p-6">
			<div class="space-y-4 text-center">
				{#if status === 'verifying'}
					<div class="mx-auto flex h-12 w-12 items-center justify-center">
						<Spinner class="h-8 w-8" />
					</div>
					<h2 class="text-xl font-semibold">Verifying your email...</h2>
					<p class="text-muted-foreground">
						Please wait while we confirm your email address.
					</p>
				{:else if status === 'success'}
					<div
						class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-green-100"
					>
						<CircleCheck class="h-6 w-6 text-green-600" />
					</div>
					<h2 class="text-xl font-semibold">Email verified!</h2>
					<p class="text-muted-foreground">
						Your email has been verified. You can now close this tab and return to the
						app.
					</p>
					<Button href="/login" class="mt-4">Go to Sign In</Button>
				{:else}
					<div
						class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-red-100"
					>
						<CircleX class="h-6 w-6 text-red-600" />
					</div>
					<h2 class="text-xl font-semibold">Verification failed</h2>
					<p class="text-muted-foreground">{errorMessage}</p>
					<Button variant="outline" href="/login" class="mt-4">Back to Sign In</Button>
				{/if}
			</div>
		</Card.Root>
	</div>
</div>
