<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import SocialAuthButtons from '$lib/components/SocialAuthButtons.svelte';
	import { authService } from '$lib/services/auth-service';
	import { auth, accessToken, currentUser } from '$lib/stores/auth';
	import { create } from '@bufbuild/protobuf';
	import {
		Provider,
		AssociateLoginTokenRequestSchema,
	} from '@eurora/shared/proto/auth_service_pb.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';

	let desktopLoginDone = $state(false);
	let pendingDesktopLogin = $state<string | null>(null);

	onMount(async () => {
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

				const isValid = await auth.ensureValidToken();
				if (isValid && get(accessToken)) {
					pendingDesktopLogin = loginToken;
				} else {
					goto('/login');
				}
				return;
			}
			loginToken = sessionStorage.getItem('loginToken');
			challengeMethod = sessionStorage.getItem('challengeMethod');
		} catch (_error) {
			goto('/login?error=invalid_login_token');
			return;
		}
	});

	async function handleConfirmDesktopLogin() {
		if (!pendingDesktopLogin) return;
		loading = true;
		submitError = null;

		const token = get(accessToken);
		if (!token) {
			submitError = 'Session expired. Please sign in again.';
			pendingDesktopLogin = null;
			loading = false;
			return;
		}

		try {
			const request = create(AssociateLoginTokenRequestSchema, {
				codeChallenge: pendingDesktopLogin,
			});
			await authService.associateLoginToken(request, token);
			sessionStorage.removeItem('loginToken');
			sessionStorage.removeItem('challengeMethod');
			desktopLoginDone = true;
			pendingDesktopLogin = null;
		} catch (err) {
			console.error('Failed to associate login token:', err);
			submitError = 'Failed to authorize desktop app. Please try again.';
		} finally {
			loading = false;
		}
	}

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
		{#if desktopLoginDone}
			<div class="text-center">
				<h1 class="text-3xl font-bold tracking-tight">Desktop app connected</h1>
				<p class="text-muted-foreground mt-2">
					You can close this tab and return to the desktop app.
				</p>
			</div>
		{:else if pendingDesktopLogin}
			<div class="text-center">
				<h1 class="text-3xl font-bold tracking-tight">Authorize desktop app</h1>
				<p class="text-muted-foreground mt-2">
					Sign in to the Eurora desktop app as <strong>{$currentUser?.email}</strong>?
				</p>
			</div>

			<Card.Root class="p-6">
				{#if submitError}
					<div class="mb-4 rounded-md bg-red-50 p-4">
						<p class="text-sm text-red-800">{submitError}</p>
					</div>
				{/if}

				<div class="flex flex-col gap-3">
					<Button class="w-full" disabled={loading} onclick={handleConfirmDesktopLogin}>
						{loading ? 'Authorizing...' : 'Authorize'}
					</Button>
					<Button
						variant="outline"
						class="w-full"
						disabled={loading}
						onclick={() => {
							auth.logout();
							pendingDesktopLogin = null;
						}}
					>
						Log out
					</Button>
				</div>
			</Card.Root>
		{:else}
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
		{/if}
	</div>
</div>
