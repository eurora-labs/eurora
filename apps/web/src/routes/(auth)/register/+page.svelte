<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { authService } from '$lib/services/auth-service';
	import { create } from '@bufbuild/protobuf';
	import { RegisterRequestSchema } from '@eurora/shared/proto/auth_service_pb.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import * as Form from '@eurora/ui/components/form/index';
	import { Input } from '@eurora/ui/components/input/index';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import EyeOffIcon from '@lucide/svelte/icons/eye-off';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import { superForm } from 'sveltekit-superforms';
	import { zodClient, type ZodObjectType } from 'sveltekit-superforms/adapters';
	import { z } from 'zod';

	const emailFromUrl = page.url.searchParams.get('email') || '';

	const registerSchema = z
		.object({
			email: z.string().email('Valid email is required'),
			username: z
				.string()
				.min(3, 'Username must be at least 3 characters')
				.max(50, 'Username must be at most 50 characters')
				.regex(
					/^[a-zA-Z0-9_]+$/,
					'Username can only contain letters, numbers, and underscores',
				),
			password: z.string().min(8, 'Password must be at least 8 characters'),
			confirmPassword: z.string().min(1, 'Please confirm your password'),
		})
		.refine((data) => data.password === data.confirmPassword, {
			message: "Passwords don't match",
			path: ['confirmPassword'],
		});

	const form = superForm(
		{ email: emailFromUrl, username: '', password: '', confirmPassword: '' },
		{
			validators: zodClient(registerSchema as unknown as ZodObjectType),
		},
	);

	const { form: formData, enhance, submitting } = form;

	let showPassword = $state(false);
	let showConfirmPassword = $state(false);
	let submitError = $state<string | null>(null);

	async function handleSubmit() {
		submitError = null;

		try {
			const registerData = create(RegisterRequestSchema, {
				username: $formData.username,
				email: $formData.email,
				password: $formData.password,
			});

			await authService.register(registerData);
			goto('/register/verify-email');
		} catch (err) {
			console.error('Registration error:', err);
			submitError =
				err instanceof Error ? err.message : 'Registration failed. Please try again.';
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

<div class="flex min-h-screen items-center justify-center px-4">
	<div class="w-full max-w-md space-y-8">
		<div class="text-center">
			<h1 class="text-3xl font-bold tracking-tight">Create your account</h1>
			<p class="text-muted-foreground mt-2">Get started with Eurora Labs</p>
		</div>

		<Card.Root class="p-6">
			<form use:enhance method="POST" onsubmit={handleSubmit} class="space-y-4">
				{#if submitError}
					<div class="rounded-md bg-red-50 p-4">
						<p class="text-sm text-red-800">{submitError}</p>
					</div>
				{/if}

				<Form.Field {form} name="email">
					<Form.Control>
						{#snippet children({ props })}
							<Form.Label>Email</Form.Label>
							<Input
								{...props}
								type="email"
								bind:value={$formData.email}
								disabled={!!emailFromUrl}
							/>
						{/snippet}
					</Form.Control>
					<Form.FieldErrors />
				</Form.Field>

				<Form.Field {form} name="username">
					<Form.Control>
						{#snippet children({ props })}
							<Form.Label>Username</Form.Label>
							<Input
								{...props}
								type="text"
								placeholder="Choose a username"
								bind:value={$formData.username}
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
									placeholder="At least 8 characters"
									bind:value={$formData.password}
									disabled={$submitting}
								/>
								<Button
									type="button"
									variant="ghost"
									size="icon-sm"
									class="absolute top-1/2 right-1.5 -translate-y-1/2"
									onclick={() => (showPassword = !showPassword)}
									disabled={$submitting}
									aria-label={showPassword ? 'Hide password' : 'Show password'}
								>
									{#if showPassword}
										<EyeOffIcon class="h-4 w-4" />
									{:else}
										<EyeIcon class="h-4 w-4" />
									{/if}
								</Button>
							</div>
						{/snippet}
					</Form.Control>
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
								<Button
									type="button"
									variant="ghost"
									size="icon-sm"
									class="absolute top-1/2 right-1.5 -translate-y-1/2"
									onclick={() => (showConfirmPassword = !showConfirmPassword)}
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
								</Button>
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
				<Button variant="link" href="/login" class="h-auto p-0 font-normal">Sign in</Button>
			</p>
		</div>
	</div>
</div>
