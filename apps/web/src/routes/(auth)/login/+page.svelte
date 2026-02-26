<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import SocialAuthButtons from '$lib/components/SocialAuthButtons.svelte';
	import { authService } from '$lib/services/auth-service';
	import { auth } from '$lib/stores/auth.js';
	import { create } from '@bufbuild/protobuf';
	import { LoginRequestSchema, Provider } from '@eurora/shared/proto/auth_service_pb.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as Separator from '@eurora/ui/components/separator/index';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import EyeOffIcon from '@lucide/svelte/icons/eye-off';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import { onMount } from 'svelte';
	import { z } from 'zod';

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

	let step = $state<'email' | 'password'>('email');
	let email = $state('');
	let password = $state('');
	let showPassword = $state(false);
	let checking = $state(false);
	let submitting = $state(false);
	let success = $state(false);
	let submitError = $state<string | null>(null);
	let emailError = $state<string | null>(null);

	async function handleEmailContinue(e: SubmitEvent) {
		e.preventDefault();
		emailError = null;
		submitError = null;

		const result = z.string().email('Please enter a valid email address').safeParse(email);
		if (!result.success) {
			emailError = result.error.issues[0].message;
			return;
		}

		checking = true;
		try {
			const check = await authService.checkEmail(email);
			switch (check.status) {
				case 'password':
					step = 'password';
					break;
				case 'oauth':
					if (check.provider === 'google') {
						await handleGoogleLogin();
					} else {
						await handleGitHubLogin();
					}
					break;
				case 'not_found':
					goto('/register?email=' + encodeURIComponent(email));
					break;
			}
		} catch (err) {
			console.error('Email check error:', err);
			submitError =
				err instanceof Error ? err.message : 'Something went wrong. Please try again.';
		} finally {
			checking = false;
		}
	}

	async function handlePasswordSubmit(e: SubmitEvent) {
		e.preventDefault();
		submitError = null;

		if (!password) {
			submitError = 'Password is required';
			return;
		}

		submitting = true;
		try {
			const loginData = create(LoginRequestSchema, {
				credential: {
					value: {
						login: email,
						password,
					},
					case: 'emailPassword',
				},
			});

			const tokens = await authService.login(loginData);
			auth.login(tokens);
			success = true;

			const redirectTo = page.url.searchParams.get('redirect') || '/settings';
			setTimeout(() => {
				goto(redirectTo);
			}, 1500);
		} catch (err) {
			console.error('Login error:', err);
			submitError = err instanceof Error ? err.message : 'Login failed. Please try again.';
		} finally {
			submitting = false;
		}
	}

	function handleBack() {
		step = 'email';
		password = '';
		submitError = null;
	}

	function storeRedirectParam() {
		const redirect = page.url.searchParams.get('redirect');
		if (redirect) {
			sessionStorage.setItem('postLoginRedirect', redirect);
		}
	}

	async function handleGoogleLogin() {
		try {
			storeRedirectParam();
			const url = (await authService.getThirdPartyAuthUrl(Provider.GOOGLE)).url;
			window.location.href = url;
		} catch (err) {
			console.error('Google login error:', err);
			submitError = err instanceof Error ? err.message : 'Login failed. Please try again.';
		}
	}

	async function handleGitHubLogin() {
		try {
			storeRedirectParam();
			const url = (await authService.getThirdPartyAuthUrl(Provider.GITHUB)).url;
			window.location.href = url;
		} catch (err) {
			console.error('GitHub login error:', err);
			submitError = err instanceof Error ? err.message : 'Login failed. Please try again.';
		}
	}

	let disabled = $derived(checking || submitting);
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

		{#if success}
			<Card.Root class="p-6">
				<div class="space-y-4 text-center">
					<div
						class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-green-100"
					>
						<svg
							class="h-6 w-6 text-green-600"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M5 13l4 4L19 7"
							></path>
						</svg>
					</div>
					<h2 class="text-xl font-semibold">Welcome back!</h2>
					<p class="text-muted-foreground">
						You have been successfully signed in. Redirecting to your dashboard...
					</p>
				</div>
			</Card.Root>
		{:else}
			<Card.Root class="p-6">
				<SocialAuthButtons
					mode="login"
					{disabled}
					onGoogle={handleGoogleLogin}
					onGitHub={handleGitHubLogin}
				/>

				<div class="relative my-6">
					<div class="absolute inset-0 flex items-center">
						<Separator.Root class="w-full" />
					</div>
					<div class="relative flex justify-center text-xs uppercase">
						<span class="bg-background text-muted-foreground px-2"
							>Or continue with</span
						>
					</div>
				</div>

				{#if submitError}
					<div class="mb-4 rounded-md bg-red-50 p-4">
						<p class="text-sm text-red-800">{submitError}</p>
					</div>
				{/if}

				{#if step === 'email'}
					<form onsubmit={handleEmailContinue} class="space-y-4">
						<div class="space-y-2">
							<Label for="email">Email</Label>
							<Input
								id="email"
								type="email"
								placeholder="Enter your email"
								bind:value={email}
								disabled={checking}
							/>
							{#if emailError}
								<p class="text-sm text-red-600">{emailError}</p>
							{/if}
						</div>

						<Button type="submit" class="w-full" disabled={checking}>
							{#if checking}
								<Loader2Icon class="mr-2 h-4 w-4 animate-spin" />
								Checking...
							{:else}
								Continue
							{/if}
						</Button>
					</form>
				{:else}
					<div class="space-y-4">
						<button
							type="button"
							class="text-muted-foreground hover:text-foreground flex items-center gap-2 text-sm transition-colors"
							onclick={handleBack}
						>
							<ArrowLeftIcon class="h-4 w-4" />
							{email}
						</button>

						<form onsubmit={handlePasswordSubmit} class="space-y-4">
							<div class="space-y-2">
								<Label for="password">Password</Label>
								<div class="relative">
									<Input
										id="password"
										type={showPassword ? 'text' : 'password'}
										placeholder="Enter your password"
										bind:value={password}
										disabled={submitting}
									/>
									<Button
										type="button"
										variant="ghost"
										size="icon-sm"
										class="absolute top-1/2 right-1.5 -translate-y-1/2"
										onclick={() => (showPassword = !showPassword)}
										{disabled}
										aria-label={showPassword
											? 'Hide password'
											: 'Show password'}
									>
										{#if showPassword}
											<EyeOffIcon class="h-4 w-4" />
										{:else}
											<EyeIcon class="h-4 w-4" />
										{/if}
									</Button>
								</div>
							</div>

							<Button type="submit" class="w-full" disabled={submitting}>
								{#if submitting}
									<Loader2Icon class="mr-2 h-4 w-4 animate-spin" />
									Signing in...
								{:else}
									Sign In
								{/if}
							</Button>
						</form>
					</div>
				{/if}
			</Card.Root>

			<div class="text-center">
				<p class="text-muted-foreground text-sm">
					Don't have an account?
					<Button variant="link" href="/register" class="h-auto p-0 font-normal">
						Create one here
					</Button>
				</p>
			</div>
		{/if}
	</div>
</div>
