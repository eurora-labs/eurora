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

	// Define form schema
	const loginSchema = z.object({
		login: z.string().min(1, 'Username or email is required'),
		password: z.string().min(1, 'Password is required')
	});

	// Initialize form with client-side validation only
	const form = superForm(
		{ login: '', password: '' },
		{
			validators: zodClient(loginSchema)
		}
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
						password: $formData.password
					},
					case: 'emailPassword'
				}
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
				<!-- Social Login Buttons -->
				<div class="space-y-3">
					<Button
						variant="outline"
						class="w-full"
						onclick={handleAppleLogin}
						disabled={$submitting}
					>
						<svg class="mr-2 h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
							<path
								d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.81-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z"
							/>
						</svg>
						Continue with Apple
					</Button>

					<Button
						variant="outline"
						class="w-full"
						onclick={handleGoogleLogin}
						disabled={$submitting}
					>
						<svg class="mr-2 h-4 w-4" viewBox="0 0 24 24">
							<path
								fill="#4285F4"
								d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
							/>
							<path
								fill="#34A853"
								d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
							/>
							<path
								fill="#FBBC05"
								d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
							/>
							<path
								fill="#EA4335"
								d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
							/>
						</svg>
						Continue with Google
					</Button>

					<Button
						variant="outline"
						class="w-full"
						onclick={handleGitHubLogin}
						disabled={$submitting}
					>
						<svg class="mr-2 h-4 w-4" fill="currentColor" viewBox="0 0 24 24">
							<path
								d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"
							/>
						</svg>
						Continue with GitHub
					</Button>
				</div>

				<!-- Divider -->
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

				<!-- Email/Password Form -->
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
										aria-label={showPassword
											? 'Hide password'
											: 'Show password'}
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
