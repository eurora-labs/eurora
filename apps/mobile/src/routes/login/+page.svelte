<script lang="ts">
	import { goto } from '$app/navigation';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import type { LoginOutcome } from '$lib/bindings/specta.bindings.js';

	const user = inject(USER_SERVICE);

	let loading = $state(false);
	let error = $state('');

	function handleOutcome(outcome: LoginOutcome) {
		switch (outcome.kind) {
			case 'success':
				goto('/');
				return;
			case 'canceled':
				loading = false;
				return;
			case 'rejected':
				error = 'Login could not be completed. Please try again.';
				loading = false;
				return;
			case 'native_unavailable':
				// signInWithGoogle already falls back to the browser
				// flow before returning this; reaching it here means
				// even the fallback was skipped, which is unexpected.
				error = 'Sign-in is unavailable on this device.';
				loading = false;
				return;
		}
	}

	async function signInWithGoogle() {
		loading = true;
		error = '';
		try {
			handleOutcome(await user.signInWithGoogle());
		} catch (err) {
			console.error('Google sign-in failed:', err);
			error = 'Sign-in failed. Please try again.';
			loading = false;
		}
	}

	async function signInWithGitHub() {
		loading = true;
		error = '';
		try {
			handleOutcome(await user.startLogin('github'));
		} catch (err) {
			console.error('GitHub sign-in failed:', err);
			error = 'Sign-in failed. Please try again.';
			loading = false;
		}
	}
</script>

<div class="flex flex-col items-center justify-center h-full px-8">
	{#if loading}
		<div class="flex flex-col items-center gap-6">
			<Spinner class="w-10 h-10" />
			<h1 class="text-xl font-semibold text-foreground">Signing you in...</h1>
			<p class="text-sm text-muted-foreground text-center">
				Complete sign-in in the secure browser sheet.
			</p>
		</div>
	{:else}
		<div class="flex flex-col items-center gap-6 w-full max-w-sm">
			<h1 class="text-2xl font-bold text-foreground">Welcome to Eurora</h1>
			<p class="text-sm text-muted-foreground text-center">Sign in to get started.</p>

			{#if error}
				<p class="text-sm text-destructive text-center">{error}</p>
			{/if}

			<Button class="w-full" onclick={signInWithGoogle}>Continue with Google</Button>
			<Button class="w-full" variant="outline" onclick={signInWithGitHub}>
				Continue with GitHub
			</Button>
		</div>
	{/if}
</div>
