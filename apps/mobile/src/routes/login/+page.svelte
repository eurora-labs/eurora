<script lang="ts">
	import { goto } from '$app/navigation';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import { onOpenUrl } from '@tauri-apps/plugin-deep-link';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { onDestroy } from 'svelte';

	const user = inject(USER_SERVICE);

	let loading = $state(false);
	let error = $state('');
	let intervalId: ReturnType<typeof setInterval> | null = null;
	let unlistenDeepLink: (() => void) | null = null;

	async function handleAuthCallback() {
		try {
			const success = await user.pollForLogin();
			if (success) {
				stopPolling();
				goto('/');
			}
		} catch {
			stopPolling();
			error = 'Login failed. Please try again.';
			loading = false;
		}
	}

	async function startLogin() {
		loading = true;
		error = '';

		try {
			unlistenDeepLink = await onOpenUrl((urls) => {
				if (urls.some((url) => url.startsWith('eurora://auth/callback'))) {
					handleAuthCallback();
				}
			});

			const loginToken = await user.getLoginToken();
			await openUrl(loginToken.url);

			intervalId = setInterval(async () => {
				try {
					const success = await user.pollForLogin();
					if (success) {
						stopPolling();
						goto('/');
					}
				} catch {
					stopPolling();
					error = 'Login failed. Please try again.';
					loading = false;
				}
			}, 5000);
		} catch {
			error = 'Failed to start login. Please try again.';
			loading = false;
		}
	}

	function stopPolling() {
		if (intervalId) {
			clearInterval(intervalId);
			intervalId = null;
		}
		if (unlistenDeepLink) {
			unlistenDeepLink();
			unlistenDeepLink = null;
		}
	}

	function cancel() {
		stopPolling();
		loading = false;
	}

	onDestroy(stopPolling);
</script>

<div class="flex flex-col items-center justify-center h-full px-8">
	{#if loading}
		<div class="flex flex-col items-center gap-6">
			<Spinner class="w-10 h-10" />
			<h1 class="text-xl font-semibold text-foreground">Waiting for you to log in...</h1>
			<p class="text-sm text-muted-foreground text-center">
				Complete sign-in in your browser, then return here.
			</p>
			<Button variant="outline" onclick={cancel}>Cancel</Button>
		</div>
	{:else}
		<div class="flex flex-col items-center gap-6 w-full max-w-sm">
			<h1 class="text-2xl font-bold text-foreground">Welcome to Eurora</h1>
			<p class="text-sm text-muted-foreground text-center">Sign in to get started.</p>

			{#if error}
				<p class="text-sm text-destructive text-center">{error}</p>
			{/if}

			<Button class="w-full" onclick={startLogin}>Log In / Sign Up</Button>
		</div>
	{/if}
</div>
