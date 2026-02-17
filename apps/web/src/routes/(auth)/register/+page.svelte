<script lang="ts">
	import SocialAuthButtons from '$lib/components/SocialAuthButtons.svelte';
	import { authService } from '$lib/services/auth-service';
	import { create } from '@bufbuild/protobuf';
	import { RegisterRequestSchema } from '@eurora/shared/proto/auth_service_pb.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import * as Form from '@eurora/ui/components/form/index';
	import { Input } from '@eurora/ui/components/input/index';
	import * as Separator from '@eurora/ui/components/separator/index';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import EyeOffIcon from '@lucide/svelte/icons/eye-off';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import { superForm } from 'sveltekit-superforms';
	import { zodClient, type ZodObjectType } from 'sveltekit-superforms/adapters';
	import { z } from 'zod';

	const registerSchema = z
		.object({
			username: z
				.string()
				.min(3, 'Username must be at least 3 characters long')
				.regex(
					/^[a-zA-Z0-9_-]+$/,
					'Username can only contain letters, numbers, hyphens, and underscores',
				),
			email: z.string().email('Please enter a valid email address'),
			displayName: z.string().optional(),
			password: z
				.string()
				.min(8, 'Password must be at least 8 characters long')
				.regex(
					/(?=.*[a-z])(?=.*[A-Z])(?=.*\d)/,
					'Password must contain at least one uppercase letter, one lowercase letter, and one number',
				),
			confirmPassword: z.string().min(1, 'Please confirm your password'),
		})
		.refine((data) => data.password === data.confirmPassword, {
			message: 'Passwords do not match',
			path: ['confirmPassword'],
		});

	const form = superForm(
		{
			username: '',
			email: '',
			displayName: '',
			password: '',
			confirmPassword: '',
		},
		{
			validators: zodClient(registerSchema as unknown as ZodObjectType),
		},
	);

	const { form: formData, enhance, submitting } = form;

	let showPassword = $state(false);
	let showConfirmPassword = $state(false);
	let success = $state(false);
	let submitError = $state<string | null>(null);

	async function handleSubmit() {
		submitError = null;

		try {
			const registerData = create(RegisterRequestSchema, {
				username: $formData.username,
				email: $formData.email,
				password: $formData.password,
				displayName: $formData.displayName || undefined,
			});

			await authService.register(registerData);
			success = true;
		} catch (err) {
			console.error('Registration error:', err);
			submitError =
				err instanceof Error ? err.message : 'Registration failed. Please try again.';
		}
	}

	function togglePasswordVisibility() {
		showPassword = !showPassword;
	}

	function toggleConfirmPasswordVisibility() {
		showConfirmPassword = !showConfirmPassword;
	}

	function handleGoogleRegister() {
		// TODO: Implement Google OAuth registration
	}

	function handleGitHubRegister() {
		// TODO: Implement GitHub OAuth registration
	}
</script>

<svelte:head>
	<title>Register - Eurora Labs</title>
	<meta
		name="description"
		content="Create your Eurora account to get started with AI-powered productivity tools."
	/>
</svelte:head>

<div class="flex min-h-screen items-center justify-center px-4">
	<div class="w-full max-w-md space-y-8">
		<div class="text-center">
			<h1 class="text-3xl font-bold tracking-tight">Create your account</h1>
			<p class="text-muted-foreground mt-2">
				Join Eurora Labs and unlock AI-powered productivity
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
					<h2 class="text-xl font-semibold">Registration successful!</h2>
					<p class="text-muted-foreground">
						Your account has been created. You can now sign in to access Eurora.
					</p>
					<Button href="/login" class="w-full">Continue to Sign In</Button>
				</div>
			</Card.Root>
		{:else}
			<Card.Root class="p-6">
				<SocialAuthButtons
					mode="register"
					disabled={$submitting}
					onGoogle={handleGoogleRegister}
					onGitHub={handleGitHubRegister}
				/>

				<div class="relative my-6">
					<div class="absolute inset-0 flex items-center">
						<Separator.Root class="w-full" />
					</div>
					<div class="relative flex justify-center text-xs uppercase">
						<span class="bg-background text-muted-foreground px-2"
							>Or register with email</span
						>
					</div>
				</div>

				<form use:enhance method="POST" onsubmit={handleSubmit} class="space-y-4">
					{#if submitError}
						<div class="rounded-md bg-red-50 p-4">
							<p class="text-sm text-red-800">{submitError}</p>
						</div>
					{/if}

					<Form.Field {form} name="username">
						<Form.Control>
							{#snippet children({ props })}
								<Form.Label>Username</Form.Label>
								<Input
									{...props}
									type="text"
									placeholder="Enter your username"
									bind:value={$formData.username}
									disabled={$submitting}
								/>
							{/snippet}
						</Form.Control>
						<Form.FieldErrors />
					</Form.Field>

					<Form.Field {form} name="email">
						<Form.Control>
							{#snippet children({ props })}
								<Form.Label>Email</Form.Label>
								<Input
									{...props}
									type="email"
									placeholder="Enter your email"
									bind:value={$formData.email}
									disabled={$submitting}
								/>
							{/snippet}
						</Form.Control>
						<Form.FieldErrors />
					</Form.Field>

					<Form.Field {form} name="displayName">
						<Form.Control>
							{#snippet children({ props })}
								<Form.Label>Display Name (Optional)</Form.Label>
								<Input
									{...props}
									type="text"
									placeholder="Enter your display name"
									bind:value={$formData.displayName}
									disabled={$submitting}
								/>
							{/snippet}
						</Form.Control>
						<Form.Description
							>This is how your name will appear to other users</Form.Description
						>
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
											<EyeOffIcon class="h-4 w-4" />
										{:else}
											<EyeIcon class="h-4 w-4" />
										{/if}
									</button>
								</div>
							{/snippet}
						</Form.Control>
						<Form.Description>
							Must be at least 8 characters with uppercase, lowercase, and number
						</Form.Description>
						<Form.FieldErrors />
					</Form.Field>

					<Form.Field {form} name="confirmPassword">
						<Form.Control>
							{#snippet children({ props })}
								<Form.Label>Confirm Password</Form.Label>
								<div class="relative">
									<Input
										{...props}
										type={showConfirmPassword ? 'text' : 'password'}
										placeholder="Confirm your password"
										bind:value={$formData.confirmPassword}
										disabled={$submitting}
									/>
									<button
										type="button"
										class="text-muted-foreground hover:text-foreground absolute top-1/2 right-3 -translate-y-1/2 transition-colors"
										onclick={toggleConfirmPasswordVisibility}
										disabled={$submitting}
										aria-label={showConfirmPassword
											? 'Hide password'
											: 'Show password'}
									>
										{#if showConfirmPassword}
											<EyeOffIcon class="h-4 w-4" />
										{:else}
											<EyeIcon class="h-4 w-4" />
										{/if}
									</button>
								</div>
							{/snippet}
						</Form.Control>
						<Form.FieldErrors />
					</Form.Field>

					<Button type="submit" class="w-full" disabled={$submitting}>
						{#if $submitting}
							<Loader2Icon class="mr-2 h-4 w-4 animate-spin" />
							Creating account...
						{:else}
							Create Account
						{/if}
					</Button>
				</form>
			</Card.Root>

			<div class="text-center">
				<p class="text-muted-foreground text-sm">
					Already have an account?
					<Button variant="link" href="/login" class="h-auto p-0 font-normal"
						>Sign in here</Button
					>
				</p>
			</div>
		{/if}
	</div>
</div>
