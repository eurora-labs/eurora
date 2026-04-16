<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import { onMount, onDestroy } from 'svelte';
	import { toast } from 'svelte-sonner';

	const user = inject(USER_SERVICE);
	const redirect = $page.url.searchParams.get('redirect') ?? '/';

	let pollId: ReturnType<typeof setInterval> | null = null;
	let cooldownId: ReturnType<typeof setInterval> | null = null;
	let resending = $state(false);
	let cooldown = $state(0);

	onMount(() => {
		pollId = setInterval(async () => {
			try {
				const verified = await user.checkVerification();
				if (verified) {
					if (pollId) clearInterval(pollId);
					goto(redirect);
				}
			} catch {
				// Silently retry on next interval
			}
		}, 3_000);
	});

	onDestroy(() => {
		if (pollId) clearInterval(pollId);
		if (cooldownId) clearInterval(cooldownId);
	});

	async function resend() {
		resending = true;
		try {
			await user.resendVerificationEmail();
			toast.success('Verification email sent!');
			cooldown = 60;
			cooldownId = setInterval(() => {
				cooldown -= 1;
				if (cooldown <= 0 && cooldownId) {
					clearInterval(cooldownId);
					cooldownId = null;
				}
			}, 1000);
		} catch (error) {
			toast.error(`Failed to resend: ${error}`);
		} finally {
			resending = false;
		}
	}

	async function tryDifferentAccount() {
		await user.logout();
		goto('/onboarding/login');
	}
</script>

<div class="flex flex-col justify-center items-center h-full px-8 gap-6">
	<div class="flex flex-col items-center gap-4 max-w-md text-center">
		<Spinner class="w-8 h-8" />
		<h1 class="text-3xl font-bold">Verify your email</h1>
		<p class="text-sm text-muted-foreground">
			We sent a verification email to <strong>{user.email}</strong>. Please check your inbox
			and click the link to continue.
		</p>
	</div>

	<div class="flex gap-4">
		<Button variant="outline" onclick={tryDifferentAccount}>Try a different account</Button>
		<Button onclick={resend} disabled={resending || cooldown > 0}>
			{#if resending}
				<Spinner class="size-4" />
				Sending...
			{:else if cooldown > 0}
				Resend ({cooldown}s)
			{:else}
				Resend email
			{/if}
		</Button>
	</div>
</div>
