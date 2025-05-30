<script lang="ts">
	import { create } from '@bufbuild/protobuf';
	import { Button, Card, Input, Label } from '@eurora/ui';
	import { Eye, EyeOff, Loader2 } from '@lucide/svelte';
	import { authService } from '$lib/services/auth-service.js';
	import { LoginRequestSchema } from '@eurora/proto/auth_service';

	let formData = $state({
		login: '',
		password: ''
	});

	let showPassword = $state(false);
	let isLoading = $state(false);
	let errors = $state<Record<string, string>>({});
	let success = $state(false);

	function validateField(field: string, value: string): string | null {
		switch (field) {
			case 'login':
				if (!value.trim()) return 'Username or email is required';
				return null;
			case 'password':
				if (!value) return 'Password is required';
				return null;
			default:
				return null;
		}
	}

	function validateForm(): boolean {
		const newErrors: Record<string, string> = {};

		// Validate all required fields
		const loginError = validateField('login', formData.login);
		if (loginError) newErrors.login = loginError;

		const passwordError = validateField('password', formData.password);
		if (passwordError) newErrors.password = passwordError;

		errors = newErrors;
		return Object.keys(newErrors).length === 0;
	}

	function handleFieldBlur(field: string, value: string) {
		const error = validateField(field, value);
		if (error) {
			errors = { ...errors, [field]: error };
		} else {
			const { [field]: _, ...rest } = errors;
			errors = rest;
		}
	}

	async function handleSubmit(event: Event) {
		event.preventDefault();

		if (!validateForm()) {
			return;
		}

		isLoading = true;

		try {
			const loginData = create(LoginRequestSchema, {
				credential: {
					value: {
						login: formData.login,
						password: formData.password
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
			errors = {
				submit: err instanceof Error ? err.message : 'Login failed. Please try again.'
			};
		} finally {
			isLoading = false;
		}
	}

	function togglePasswordVisibility() {
		showPassword = !showPassword;
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
			<p class="mt-2 text-muted-foreground">
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
				<form onsubmit={handleSubmit} class="space-y-4">
					{#if errors.submit}
						<div class="rounded-md bg-red-50 p-4">
							<p class="text-sm text-red-800">{errors.submit}</p>
						</div>
					{/if}

					<div class="space-y-2">
						<Label for="login">Username or Email</Label>
						<Input
							id="login"
							type="text"
							placeholder="Enter your username or email"
							bind:value={formData.login}
							onblur={() => handleFieldBlur('login', formData.login)}
							disabled={isLoading}
							class={errors.login ? 'border-red-500 focus:border-red-500' : ''}
							required
						/>
						{#if errors.login}
							<p class="text-sm text-red-600">{errors.login}</p>
						{/if}
					</div>

					<div class="space-y-2">
						<Label for="password">Password</Label>
						<div class="relative">
							<Input
								id="password"
								type={showPassword ? 'text' : 'password'}
								placeholder="Enter your password"
								bind:value={formData.password}
								onblur={() => handleFieldBlur('password', formData.password)}
								disabled={isLoading}
								class={errors.password ? 'border-red-500 focus:border-red-500' : ''}
								required
							/>
							<button
								type="button"
								class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground transition-colors hover:text-foreground"
								onclick={togglePasswordVisibility}
								disabled={isLoading}
								aria-label={showPassword ? 'Hide password' : 'Show password'}
							>
								{#if showPassword}
									<EyeOff class="h-4 w-4" />
								{:else}
									<Eye class="h-4 w-4" />
								{/if}
							</button>
						</div>
						{#if errors.password}
							<p class="text-sm text-red-600">{errors.password}</p>
						{/if}
					</div>

					<Button
						type="submit"
						class="w-full"
						disabled={isLoading || Object.keys(errors).length > 0}
					>
						{#if isLoading}
							<Loader2 class="mr-2 h-4 w-4 animate-spin" />
							Signing in...
						{:else}
							Sign In
						{/if}
					</Button>
				</form>
			</Card.Root>

			<div class="text-center">
				<p class="text-sm text-muted-foreground">
					Don't have an account?
					<Button variant="link" href="/register" class="h-auto p-0 font-normal">
						Create one here
					</Button>
				</p>
			</div>
		{/if}
	</div>
</div>
