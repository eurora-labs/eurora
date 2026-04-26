<script lang="ts">
	import { goto } from '$app/navigation';
	import SocialAuthButtons from '$lib/components/SocialAuthButtons.svelte';
	import { AUTH_SERVICE } from '$lib/services/auth-service.js';
	import { auth } from '$lib/stores/auth.js';
	import { create } from '@bufbuild/protobuf';
	import { inject } from '@eurora/shared/context';
	import {
		Provider,
		RegisterRequestSchema,
		CheckEmailRequestSchema,
	} from '@eurora/shared/proto/auth_service_pb.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';

	const authService = inject(AUTH_SERVICE);

	let loading = $state(false);
	let submitError = $state<string | null>(null);
	let email = $state('');
	let password = $state('');
	let showRegisterFields = $state(false);

	async function handleEmailContinue() {
		if (!email.trim()) return;
		loading = true;
		submitError = null;
		try {
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
			showRegisterFields = true;
		} catch (err) {
			console.error('Check email error:', err);
			submitError =
				err instanceof Error ? err.message : 'Something went wrong. Please try again.';
		} finally {
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
			const associated = await authService.associateAppLoginIfPending(tokens.accessToken, {
				consumeRedirect: true,
			});
			if (associated) return;
			goto('/');
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
			const url = (await authService.getThirdPartyAuthUrl(Provider.GOOGLE)).url;
			window.location.href = url;
		} catch (err) {
			console.error('Google registration error:', err);
			submitError =
				err instanceof Error ? err.message : 'Registration failed. Please try again.';
			loading = false;
		}
	}

	async function handleGitHubLogin() {
		loading = true;
		submitError = null;
		try {
			const url = (await authService.getThirdPartyAuthUrl(Provider.GITHUB)).url;
			window.location.href = url;
		} catch (err) {
			console.error('GitHub registration error:', err);
			submitError =
				err instanceof Error ? err.message : 'Registration failed. Please try again.';
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Create Account - Eurora Labs</title>
	<meta
		name="description"
		content="Create your Eurora account to get started with AI-powered productivity tools."
	/>
</svelte:head>

<div class="flex min-h-screen items-start justify-center px-4 pt-[25vh]">
	<div class="w-full max-w-md space-y-8">
		<div class="text-center">
			<h1 class="text-3xl font-bold tracking-tight">Create your account</h1>
			<p class="text-muted-foreground mt-2">Get started with Eurora Labs</p>
		</div>

		<Card.Root class="p-6">
			{#if submitError}
				<div class="mb-4 rounded-md bg-red-50 p-4">
					<p class="text-sm text-red-800">{submitError}</p>
				</div>
			{/if}

			<SocialAuthButtons
				mode="register"
				disabled={loading}
				onGoogle={handleGoogleLogin}
				onGitHub={handleGitHubLogin}
			/>

			{#if showRegisterFields}
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
							showRegisterFields = false;
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
						autocomplete="email"
					/>
					<Button
						type="submit"
						class="w-full"
						disabled={loading ||
							!email.includes('@') ||
							!email.split('@')[1]?.includes('.')}
					>
						{loading ? 'Checking...' : 'Continue with email'}
					</Button>
				</form>
			{/if}
		</Card.Root>

		<p class="text-muted-foreground text-center text-sm">
			Already have an account?
			<Button variant="link" href="/login" class="h-auto p-0 font-normal">Sign in</Button>
		</p>
	</div>
</div>
