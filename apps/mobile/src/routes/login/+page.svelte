<script lang="ts">
	import { goto } from '$app/navigation';
	import { OAuthSessionError, authenticateOAuthSession } from '$lib/services/oauth-session.js';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';

	const user = inject(USER_SERVICE);

	const CALLBACK_SCHEME = 'eurora';
	const REDIRECT_URI = `${CALLBACK_SCHEME}://mobile/callback`;

	let loading = $state(false);
	let error = $state('');

	async function startLogin() {
		loading = true;
		error = '';

		try {
			const loginToken = await user.getLoginToken(REDIRECT_URI);

			const { url: callbackUrl } = await authenticateOAuthSession({
				authUrl: loginToken.url,
				callbackScheme: CALLBACK_SCHEME,
			});

			const success = await user.completeLogin(callbackUrl);
			if (success) {
				goto('/');
			} else {
				error = 'Login could not be completed. Please try again.';
				loading = false;
			}
		} catch (err) {
			if (err instanceof OAuthSessionError && err.code === 'USER_CANCELED') {
				loading = false;
				return;
			}
			console.error('Login failed:', err);
			error = 'Login failed. Please try again.';
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

			<Button class="w-full" onclick={startLogin}>Log In / Sign Up</Button>
		</div>
	{/if}
</div>
