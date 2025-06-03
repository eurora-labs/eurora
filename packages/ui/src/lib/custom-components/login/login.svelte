<script lang="ts">
	import * as Card from '$lib/components/card/index.js';
	import { SocialAuthButtons } from '$lib/custom-components/social-auth-buttons/index.js';
	import * as Separator from '$lib/components/separator/index.js';
	import * as Form from '$lib/components/form/index.js';
	import { Button } from '$lib/components/button/index.js';
	import { Input } from '$lib/components/input/index.js';
	import { Eye, EyeOff, Loader2 } from '@lucide/svelte';
	import { superForm } from 'sveltekit-superforms';
	import { zodClient } from 'sveltekit-superforms/adapters';
	import { z } from 'zod';

	export interface Props {
		onsubmit: (event: SubmitEvent) => void;
		submitError: string | null;

		onApple: () => void;
		onGoogle: () => void;
		onGitHub: () => void;
	}

	let { onsubmit, submitError, onApple, onGoogle, onGitHub }: Props = $props();

	// Define form schema
	const loginSchema = z.object({
		login: z.string().min(1, 'Username or email is required'),
		password: z.string().min(1, 'Password is required'),
	});

	// Initialize form with client-side validation only
	const form = superForm(
		{ login: '', password: '' },
		{
			validators: zodClient(loginSchema as any),
		},
	);

	const { form: formData, enhance, errors, submitting } = form;

	let showPassword = $state(false);

	function togglePasswordVisibility() {
		showPassword = !showPassword;
	}
</script>

<Card.Root class="p-6">
	<SocialAuthButtons mode="login" disabled={$submitting} {onApple} {onGoogle} {onGitHub} />

	<div class="relative my-6">
		<div class="absolute inset-0 flex items-center">
			<Separator.Root class="w-full" />
		</div>
		<div class="relative flex justify-center text-xs uppercase">
			<span class="bg-background text-muted-foreground px-2">Or continue with</span>
		</div>
	</div>

	<form use:enhance {onsubmit} class="space-y-4">
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
