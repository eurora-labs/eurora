<script lang="ts">
	import { Button, Card, Input, Label } from '@eurora/ui';
	import { Eye, EyeOff, Loader2 } from '@lucide/svelte';

	let formData = $state({
		username: '',
		email: '',
		password: '',
		confirmPassword: '',
		displayName: ''
	});

	let showPassword = $state(false);
	let showConfirmPassword = $state(false);
	let isLoading = $state(false);
	let errors = $state<Record<string, string>>({});
	let success = $state(false);

	function validateField(field: string, value: string): string | null {
		switch (field) {
			case 'username':
				if (!value.trim()) return 'Username is required';
				if (value.length < 3) return 'Username must be at least 3 characters long';
				if (!/^[a-zA-Z0-9_-]+$/.test(value))
					return 'Username can only contain letters, numbers, hyphens, and underscores';
				return null;
			case 'email':
				if (!value.trim()) return 'Email is required';
				if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value))
					return 'Please enter a valid email address';
				return null;
			case 'password':
				if (!value) return 'Password is required';
				if (value.length < 8) return 'Password must be at least 8 characters long';
				if (!/(?=.*[a-z])(?=.*[A-Z])(?=.*\d)/.test(value))
					return 'Password must contain at least one uppercase letter, one lowercase letter, and one number';
				return null;
			case 'confirmPassword':
				if (!value) return 'Please confirm your password';
				if (value !== formData.password) return 'Passwords do not match';
				return null;
			default:
				return null;
		}
	}

	function validateForm(): boolean {
		const newErrors: Record<string, string> = {};

		// Validate all required fields
		const usernameError = validateField('username', formData.username);
		if (usernameError) newErrors.username = usernameError;

		const emailError = validateField('email', formData.email);
		if (emailError) newErrors.email = emailError;

		const passwordError = validateField('password', formData.password);
		if (passwordError) newErrors.password = passwordError;

		const confirmPasswordError = validateField('confirmPassword', formData.confirmPassword);
		if (confirmPasswordError) newErrors.confirmPassword = confirmPasswordError;

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
			// TODO: Implement actual registration API call
			// This would typically call the auth service register endpoint
			console.log('Registration data:', {
				username: formData.username,
				email: formData.email,
				password: formData.password,
				display_name: formData.displayName || undefined
			});

			// Simulate API call
			await new Promise((resolve) => setTimeout(resolve, 1500));

			success = true;
		} catch (err) {
			errors = {
				submit:
					err instanceof Error ? err.message : 'Registration failed. Please try again.'
			};
		} finally {
			isLoading = false;
		}
	}

	function togglePasswordVisibility() {
		showPassword = !showPassword;
	}

	function toggleConfirmPasswordVisibility() {
		showConfirmPassword = !showConfirmPassword;
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
			<p class="mt-2 text-muted-foreground">
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
				<form onsubmit={handleSubmit} class="space-y-4">
					{#if errors.submit}
						<div class="rounded-md bg-red-50 p-4">
							<p class="text-sm text-red-800">{errors.submit}</p>
						</div>
					{/if}

					<div class="space-y-2">
						<Label for="username">Username</Label>
						<Input
							id="username"
							type="text"
							placeholder="Enter your username"
							bind:value={formData.username}
							onblur={() => handleFieldBlur('username', formData.username)}
							disabled={isLoading}
							class={errors.username ? 'border-red-500 focus:border-red-500' : ''}
							required
						/>
						{#if errors.username}
							<p class="text-sm text-red-600">{errors.username}</p>
						{/if}
					</div>

					<div class="space-y-2">
						<Label for="email">Email</Label>
						<Input
							id="email"
							type="email"
							placeholder="Enter your email"
							bind:value={formData.email}
							onblur={() => handleFieldBlur('email', formData.email)}
							disabled={isLoading}
							class={errors.email ? 'border-red-500 focus:border-red-500' : ''}
							required
						/>
						{#if errors.email}
							<p class="text-sm text-red-600">{errors.email}</p>
						{/if}
					</div>

					<div class="space-y-2">
						<Label for="displayName">Display Name (Optional)</Label>
						<Input
							id="displayName"
							type="text"
							placeholder="Enter your display name"
							bind:value={formData.displayName}
							disabled={isLoading}
						/>
						<p class="text-xs text-muted-foreground">
							This is how your name will appear to other users
						</p>
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
						{:else}
							<p class="text-xs text-muted-foreground">
								Must be at least 8 characters with uppercase, lowercase, and number
							</p>
						{/if}
					</div>

					<div class="space-y-2">
						<Label for="confirmPassword">Confirm Password</Label>
						<div class="relative">
							<Input
								id="confirmPassword"
								type={showConfirmPassword ? 'text' : 'password'}
								placeholder="Confirm your password"
								bind:value={formData.confirmPassword}
								onblur={() =>
									handleFieldBlur('confirmPassword', formData.confirmPassword)}
								disabled={isLoading}
								class={errors.confirmPassword
									? 'border-red-500 focus:border-red-500'
									: ''}
								required
							/>
							<button
								type="button"
								class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground transition-colors hover:text-foreground"
								onclick={toggleConfirmPasswordVisibility}
								disabled={isLoading}
								aria-label={showConfirmPassword ? 'Hide password' : 'Show password'}
							>
								{#if showConfirmPassword}
									<EyeOff class="h-4 w-4" />
								{:else}
									<Eye class="h-4 w-4" />
								{/if}
							</button>
						</div>
						{#if errors.confirmPassword}
							<p class="text-sm text-red-600">{errors.confirmPassword}</p>
						{/if}
					</div>

					<Button
						type="submit"
						class="w-full"
						disabled={isLoading || Object.keys(errors).length > 0}
					>
						{#if isLoading}
							<Loader2 class="mr-2 h-4 w-4 animate-spin" />
							Creating account...
						{:else}
							Create Account
						{/if}
					</Button>
				</form>
			</Card.Root>

			<div class="text-center">
				<p class="text-sm text-muted-foreground">
					Already have an account?
					<Button variant="link" href="/login" class="h-auto p-0 font-normal">
						Sign in here
					</Button>
				</p>
			</div>
		{/if}
	</div>
</div>
