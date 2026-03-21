<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import SocialAuthButtons from '$lib/components/SocialAuthButtons.svelte';
	import { AUTH_SERVICE } from '$lib/services/auth-service.js';
	import { auth, accessToken, currentUser } from '$lib/stores/auth.js';
	import { create } from '@bufbuild/protobuf';
	import { inject } from '@eurora/shared/context';
	import {
		Provider,
		AssociateLoginTokenRequestSchema,
		LoginRequestSchema,
		CheckEmailRequestSchema,
		RegisterRequestSchema,
	} from '@eurora/shared/proto/auth_service_pb.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';

	let desktopLoginDone = $state(false);
	const authService = inject(AUTH_SERVICE);
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
	let email = $state('');
	let password = $state('');
	let showPassword = $state(false);
	let showRegister = $state(false);

	function storeRedirectParam() {
		const redirect = page.url.searchParams.get('redirect');
		if (redirect && redirect.startsWith('/') && !redirect.startsWith('//')) {
			sessionStorage.setItem('postLoginRedirect', redirect);
		}
	}

	async function handleEmailContinue() {
		if (!email.trim()) return;
		loading = true;
		submitError = null;
		try {
			storeRedirectParam();
			const resp = await authService.checkEmail(
				create(CheckEmailRequestSchema, { email: email.trim() }),
			);
			if (resp.status === 'oauth' && resp.provider !== null) {
				const provider =
					resp.provider === Provider.GOOGLE ? Provider.GOOGLE : Provider.GITHUB;
				const url = (await authService.getThirdPartyAuthUrl(provider)).url;
				window.location.href = url;
				return;
			}
			if (resp.status === 'not_found') {
				showRegister = true;
				return;
			}
			showPassword = true;
		} catch (err) {
			console.error('Check email error:', err);
			submitError =
				err instanceof Error ? err.message : 'Something went wrong. Please try again.';
		} finally {
			loading = false;
		}
	}

	async function handleEmailPasswordLogin() {
		if (!email.trim() || !password) return;
		loading = true;
		submitError = null;
		try {
			const request = create(LoginRequestSchema, {
				credential: {
					case: 'emailPassword',
					value: { login: email.trim(), password },
				},
			});
			const tokens = await authService.login(request);
			auth.login(tokens);
			const redirect = sessionStorage.getItem('postLoginRedirect');
			sessionStorage.removeItem('postLoginRedirect');
			goto(redirect || '/');
		} catch (err) {
			console.error('Email/password login error:', err);
			submitError = err instanceof Error ? err.message : 'Invalid email or password.';
			loading = false;
		}
	}

	async function handleRegister() {
		if (!email.trim() || !password) return;
		loading = true;
		submitError = null;
		try {
			const request = create(RegisterRequestSchema, {
				email: email.trim(),
				password,
			});
			const tokens = await authService.register(request);
			auth.login(tokens);
			const redirect = sessionStorage.getItem('postLoginRedirect');
			sessionStorage.removeItem('postLoginRedirect');
			goto(redirect || '/');
		} catch (err) {
			console.error('Registration error:', err);
			submitError =
				err instanceof Error ? err.message : 'Registration failed. Please try again.';
			loading = false;
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

<div class="flex min-h-screen items-start justify-center px-4 pt-[25vh]">
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
			<div class="text-left">
				<h1 class="text-3xl font-bold tracking-tight">Welcome to Eurora</h1>
				<p class="text-muted-foreground mt-2">A better, easier way to use AI.</p>
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

				{#if showRegister}
					<form
						class="mt-3 space-y-4"
						onsubmit={(e) => {
							e.preventDefault();
							handleRegister();
						}}
					>
						<Input
							id="email"
							type="email"
							placeholder="Email"
							bind:value={email}
							disabled={loading}
							autocomplete="email"
						/>
						<div>
							<Input
								id="password"
								type="password"
								placeholder="Password"
								bind:value={password}
								disabled={loading}
								autocomplete="new-password"
							/>
							<p class="text-muted-foreground mt-1 text-xs">
								Must be at least 8 characters
							</p>
						</div>
						<Button
							type="submit"
							class="w-full"
							disabled={loading || !email.trim() || !password}
						>
							{loading ? 'Creating account...' : 'Create account'}
						</Button>
						<Button
							type="button"
							variant="ghost"
							class="w-full"
							disabled={loading}
							onclick={() => {
								showRegister = false;
								password = '';
								submitError = null;
							}}
						>
							Back
						</Button>
					</form>
				{:else if showPassword}
					<form
						class="mt-3 space-y-4"
						onsubmit={(e) => {
							e.preventDefault();
							handleEmailPasswordLogin();
						}}
					>
						<Input
							id="email"
							type="email"
							placeholder="Email"
							bind:value={email}
							disabled={loading}
							autocomplete="username"
						/>
						<Input
							id="password"
							type="password"
							placeholder="Password"
							bind:value={password}
							disabled={loading}
							autocomplete="current-password"
						/>
						<Button
							type="submit"
							class="w-full"
							disabled={loading || !email.trim() || !password}
						>
							{loading ? 'Signing in...' : 'Sign in'}
						</Button>
						<Button
							type="button"
							variant="ghost"
							class="w-full"
							disabled={loading}
							onclick={() => {
								showPassword = false;
								password = '';
								submitError = null;
							}}
						>
							Back
						</Button>
					</form>
				{:else}
					<form
						class="mt-3 space-y-4"
						onsubmit={(e) => {
							e.preventDefault();
							handleEmailContinue();
						}}
					>
						<Input
							id="email"
							type="email"
							placeholder="Email"
							bind:value={email}
							disabled={loading}
							autocomplete="username"
						/>
						<Button
							type="submit"
							class="w-full"
							variant="outline"
							disabled={loading ||
								!email.includes('@') ||
								!email.split('@')[1]?.includes('.')}
						>
							{loading ? 'Checking...' : 'Continue'}
						</Button>
					</form>
				{/if}
			</Card.Root>

			<p class="text-muted-foreground text-center text-sm">
				Don't have an account?
				<Button variant="link" href="/register" class="h-auto p-0 font-normal">
					Create one
				</Button>
			</p>
		{/if}
	</div>
</div>
