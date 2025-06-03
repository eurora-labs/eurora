<script lang="ts">
	import { create } from '@bufbuild/protobuf';
	import * as Form from '@eurora/ui/components/form/index';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';
	import * as Separator from '@eurora/ui/components/separator/index';
	import { Eye, EyeOff, Loader2 } from '@lucide/svelte';
	import { authService } from '$lib/services/auth-service.js';
	import { LoginRequestSchema } from '@eurora/proto/auth_service';
	import { superForm } from 'sveltekit-superforms';
	import { zodClient } from 'sveltekit-superforms/adapters';
	import { z } from 'zod';
	import SocialAuthButtons from '$lib/components/SocialAuthButtons.svelte';

	// Define form schema
	const loginSchema = z.object({
		login: z.string().min(1, 'Username or email is required'),
		password: z.string().min(1, 'Password is required'),
	});

	// Initialize form with client-side validation only
	const form = superForm(
		{ login: '', password: '' },
		{
			validators: zodClient(loginSchema),
		},
	);

	const { form: formData, enhance, errors, submitting } = form;

	let showPassword = $state(false);
	let success = $state(false);
	let submitError = $state<string | null>(null);

	async function handleSubmit() {
		submitError = null;

		try {
			const loginData = create(LoginRequestSchema, {
				credential: {
					value: {
						login: $formData.login,
						password: $formData.password,
					},
					case: 'emailPassword',
				},
			});

			console.log('Logging in user:', loginData);

			// Call the auth service to login the user
			const tokens = await authService.login(loginData);

			console.log('Login successful, tokens:', tokens);
			success = true;

			// // Redirect to dashboard or home page after a short delay
			// setTimeout(() => {
			// 	window.location.href = '/app';
			// }, 1500);
		} catch (err) {
			console.error('Login error:', err);
			submitError = err instanceof Error ? err.message : 'Login failed. Please try again.';
		}
	}

	function togglePasswordVisibility() {
		showPassword = !showPassword;
	}

	// Social login handlers
	function handleAppleLogin() {
		console.log('Apple login clicked');
		// TODO: Implement Apple OAuth
	}

	function handleGoogleLogin() {
		console.log('Google login clicked');
		// TODO: Implement Google OAuth
	}

	function handleGitHubLogin() {
		console.log('GitHub login clicked');
		// TODO: Implement GitHub OAuth
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
			<p class="text-muted-foreground mt-2">Sign in to your account to continue with Eurora Labs</p>
		</div>

		{#if success}
			<Card.Root class="p-6">
				<div class="space-y-4 text-center">
					<div class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-green-100">
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
					onApple={handleAppleLogin}
					onGoogle={handleGoogleLogin}
					onGitHub={handleGitHubLogin}
				/>

				<div class="relative my-6">
					<div class="absolute inset-0 flex items-center">
						<Separator.Root class="w-full" />
					</div>
					<div class="relative flex justify-center text-xs uppercase">
						<span class="bg-background text-muted-foreground px-2">Or continue with</span>
					</div>
				</div>

				<form use:enhance onsubmit={handleSubmit} class="space-y-4">
					{#if submitError}
						<div class="rounded-md bg-red-50 p-4">
							<p class="text-sm text-red-800">{submitError}</p>
						</div>
					{/if}

					<Form.Field {form} name="login">
						<Form.Control>
							{#snippet children({ props })}
								<Form.Label>Username or Email</Form.Label>
								<Input
									{...props}
									type="text"
									placeholder="Enter your username or email"
									bind:value={$formData.login}
									disabled={$submitting}
								/>
							{/snippet}
						</Form.Control>
						<Form.FieldErrors />
					</Form.Field>

					<Form.Field {form} name="password">
						<Form.Control>
							{#snippet children({ props })}
								<Form.Label>Password</Form.Label>
								<div class="relative">
									<Input
										{...props}
										type={showPassword ? 'text' : 'password'}
										placeholder="Enter your password"
										bind:value={$formData.password}
										disabled={$submitting}
									/>
									<button
										type="button"
										class="text-muted-foreground hover:text-foreground absolute top-1/2 right-3 -translate-y-1/2 transition-colors"
										onclick={togglePasswordVisibility}
										disabled={$submitting}
										aria-label={showPassword ? 'Hide password' : 'Show password'}
									>
										{#if showPassword}
											<EyeOff class="h-4 w-4" />
										{:else}
											<Eye class="h-4 w-4" />
										{/if}
									</button>
								</div>
							{/snippet}
						</Form.Control>
						<Form.FieldErrors />
					</Form.Field>

					<Button type="submit" class="w-full" disabled={$submitting}>
						{#if $submitting}
							<Loader2 class="mr-2 h-4 w-4 animate-spin" />
							Signing in...
						{:else}
							Sign In
						{/if}
					</Button>
				</form>
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
