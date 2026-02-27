<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import SocialAuthButtons from '$lib/components/SocialAuthButtons.svelte';
	import { authService } from '$lib/services/auth-service';
	import { Provider } from '@eurora/shared/proto/auth_service_pb.js';
	import * as Card from '@eurora/ui/components/card/index';
	import { onMount } from 'svelte';

	onMount(() => {
		try {
			let loginToken = page.url.searchParams.get('code_challenge');
			let challengeMethod = page.url.searchParams.get('code_challenge_method');
			if (loginToken && challengeMethod) {
				if (loginToken.length !== 43 || challengeMethod !== 'S256') {
					console.error('Invalid login token or challenge method');
					goto('/login?error=invalid_login_token');
					return;
				}
				sessionStorage.setItem('loginToken', loginToken);
				sessionStorage.setItem('challengeMethod', challengeMethod);
				goto('/login');
				return;
			}
			loginToken = sessionStorage.getItem('loginToken');
			challengeMethod = sessionStorage.getItem('challengeMethod');
		} catch (_error) {
			goto('/login?error=invalid_login_token');
			return;
		}
	});

	let loading = $state(false);
	let submitError = $state<string | null>(null);

	function storeRedirectParam() {
		const redirect = page.url.searchParams.get('redirect');
		if (redirect) {
			sessionStorage.setItem('postLoginRedirect', redirect);
		}
	}

	async function handleGoogleLogin() {
		loading = true;
		submitError = null;
		try {
			storeRedirectParam();
			const url = (await authService.getThirdPartyAuthUrl(Provider.GOOGLE)).url;
			window.location.href = url;
		} catch (err) {
			console.error('Google login error:', err);
			submitError = err instanceof Error ? err.message : 'Login failed. Please try again.';
			loading = false;
		}
	}

	async function handleGitHubLogin() {
		loading = true;
		submitError = null;
		try {
			storeRedirectParam();
			const url = (await authService.getThirdPartyAuthUrl(Provider.GITHUB)).url;
			window.location.href = url;
		} catch (err) {
			console.error('GitHub login error:', err);
			submitError = err instanceof Error ? err.message : 'Login failed. Please try again.';
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Sign In - Eurora Labs</title>
	<meta
		name="description"
		content="Sign in to your Eurora account to access AI-powered productivity tools."
	/>
</svelte:head>

<div class="flex min-h-screen items-center justify-center px-4">
	<div class="w-full max-w-md space-y-8">
		<div class="text-center">
			<h1 class="text-3xl font-bold tracking-tight">Welcome back</h1>
			<p class="text-muted-foreground mt-2">
				Sign in to your account to continue with Eurora Labs
			</p>
		</div>

		<Card.Root class="p-6">
			{#if submitError}
				<div class="mb-4 rounded-md bg-red-50 p-4">
					<p class="text-sm text-red-800">{submitError}</p>
				</div>
			{/if}

			<SocialAuthButtons
				mode="login"
				disabled={loading}
				onGoogle={handleGoogleLogin}
				onGitHub={handleGitHubLogin}
			/>
		</Card.Root>

		<p class="text-muted-foreground text-center text-sm">
			Email &amp; password login is coming soon.
		</p>
	</div>
</div>
