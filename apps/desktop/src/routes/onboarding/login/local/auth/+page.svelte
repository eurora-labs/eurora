<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import * as Tabs from '@eurora/ui/components/tabs/index';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);

	let mode: 'login' | 'register' = $state('login');
	let submitting = $state(false);

	let login = $state('');
	let password = $state('');

	let regUsername = $state('');
	let regEmail = $state('');
	let regPassword = $state('');

	async function handleLogin() {
		submitting = true;
		try {
			await taurpc.auth.login(login, password);
			goto('/');
		} catch (error) {
			toast.error(`Login failed: ${error}`);
		} finally {
			submitting = false;
		}
	}

	async function handleRegister() {
		submitting = true;
		try {
			await taurpc.auth.register(regUsername, regEmail, regPassword);
			goto('/');
		} catch (error) {
			toast.error(`Registration failed: ${error}`);
		} finally {
			submitting = false;
		}
	}
</script>

<div class="flex flex-col justify-center h-full px-8 gap-6">
	<div>
		<h1 class="text-3xl font-bold mb-2">Local Account</h1>
		<p class="text-sm text-muted-foreground">
			Sign in or create an account on your self-hosted backend.
		</p>
	</div>

	<Tabs.Root bind:value={mode}>
		<Tabs.List class="w-full">
			<Tabs.Trigger value="login" class="flex-1">Sign In</Tabs.Trigger>
			<Tabs.Trigger value="register" class="flex-1">Register</Tabs.Trigger>
		</Tabs.List>

		<Tabs.Content value="login" class="flex flex-col gap-4 pt-4">
			<div class="flex flex-col gap-2">
				<Label for="login" class="text-sm font-medium">Username or Email</Label>
				<Input
					id="login"
					placeholder="Enter your username or email"
					bind:value={login}
					disabled={submitting}
				/>
			</div>
			<div class="flex flex-col gap-2">
				<Label for="login-password" class="text-sm font-medium">Password</Label>
				<Input
					id="login-password"
					type="password"
					placeholder="Enter your password"
					bind:value={password}
					disabled={submitting}
				/>
			</div>
			<Button class="w-full" onclick={handleLogin} disabled={submitting}>
				{#if submitting}
					<Spinner class="size-4" />
					Signing in...
				{:else}
					Sign In
				{/if}
			</Button>
		</Tabs.Content>

		<Tabs.Content value="register" class="flex flex-col gap-4 pt-4">
			<div class="flex flex-col gap-2">
				<Label for="reg-username" class="text-sm font-medium">Username</Label>
				<Input
					id="reg-username"
					placeholder="Choose a username"
					bind:value={regUsername}
					disabled={submitting}
				/>
			</div>
			<div class="flex flex-col gap-2">
				<Label for="reg-email" class="text-sm font-medium">Email</Label>
				<Input
					id="reg-email"
					type="email"
					placeholder="Enter your email"
					bind:value={regEmail}
					disabled={submitting}
				/>
			</div>
			<div class="flex flex-col gap-2">
				<Label for="reg-password" class="text-sm font-medium">Password</Label>
				<Input
					id="reg-password"
					type="password"
					placeholder="Choose a password"
					bind:value={regPassword}
					disabled={submitting}
				/>
			</div>
			<Button class="w-full" onclick={handleRegister} disabled={submitting}>
				{#if submitting}
					<Spinner class="size-4" />
					Creating account...
				{:else}
					Create Account
				{/if}
			</Button>
		</Tabs.Content>
	</Tabs.Root>

	<div>
		<Button
			variant="outline"
			onclick={() => goto('/onboarding/login/local')}
			disabled={submitting}>Back</Button
		>
	</div>
</div>
