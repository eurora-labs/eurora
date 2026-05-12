<script lang="ts">
	import { goto } from '$app/navigation';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import { SiApple, SiGithub, SiGoogle } from '@icons-pack/svelte-simple-icons';
	import MoreHorizontalIcon from '@lucide/svelte/icons/more-horizontal';
	import type { LoginOutcome } from '$lib/bindings/specta.bindings.js';

	const user = inject(USER_SERVICE);

	let loading = $state(false);
	let error = $state('');

	function handleOutcome(outcome: LoginOutcome) {
		switch (outcome.kind) {
			case 'success':
				goto('/');
				return;
			case 'canceled':
				loading = false;
				return;
			case 'rejected':
				console.warn('Sign-in rejected:', outcome.reason);
				error = 'Login could not be completed. Please try again.';
				loading = false;
				return;
			case 'native_unavailable':
				error = 'Sign-in is unavailable on this device.';
				loading = false;
				return;
		}
	}

	async function signInWithGoogle() {
		loading = true;
		error = '';
		try {
			handleOutcome(await user.signInWithGoogle());
		} catch (err) {
			console.error('Google sign-in failed:', err);
			error = 'Sign-in failed. Please try again.';
			loading = false;
		}
	}

	async function signInWithApple() {
		loading = true;
		error = '';
		try {
			handleOutcome(await user.signInWithApple());
		} catch (err) {
			console.error('Apple sign-in failed:', err);
			error = 'Sign-in failed. Please try again.';
			loading = false;
		}
	}

	async function signInWithGitHub() {
		loading = true;
		error = '';
		try {
			handleOutcome(await user.startLogin('github'));
		} catch (err) {
			console.error('GitHub sign-in failed:', err);
			error = 'Sign-in failed. Please try again.';
			loading = false;
		}
	}
</script>

<div
	class="flex h-full flex-col px-6 pt-[env(safe-area-inset-top)] pb-[max(env(safe-area-inset-bottom),1.5rem)]"
>
	<header class="flex shrink-0 items-center justify-end py-2">
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button {...props} variant="ghost" size="icon" aria-label="More options">
						<MoreHorizontalIcon />
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content align="end">
				<DropdownMenu.Item class="cursor-pointer" onclick={() => goto('/login/advanced')}>
					Advanced
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</header>

	{#if loading}
		<div class="flex flex-1 flex-col items-center justify-center gap-6">
			<Spinner class="w-10 h-10" />
			<h1 class="text-xl font-semibold text-foreground">Signing you in...</h1>
			<p class="text-sm text-muted-foreground text-center">
				Complete sign-in in the secure browser sheet.
			</p>
		</div>
	{:else}
		<div class="flex flex-1 flex-col items-center justify-center gap-3">
			<EuroraLogo size="128" />
			<p class="text-sm text-muted-foreground">Designed in The Netherlands</p>
		</div>

		<div class="flex flex-col">
			{#if error}
				<p class="text-sm text-destructive text-center mb-4">{error}</p>
			{/if}

			<div class="flex flex-col gap-2">
				<Button class="w-full" size="lg" onclick={signInWithApple}>
					<SiApple />
					Continue with Apple
				</Button>
				<Button class="w-full" size="lg" variant="outline" onclick={signInWithGoogle}>
					<SiGoogle />
					Continue with Google
				</Button>
				<Button class="w-full" size="lg" variant="outline" onclick={signInWithGitHub}>
					<SiGithub />
					Continue with GitHub
				</Button>
				<Button
					class="w-full"
					size="lg"
					variant="outline"
					onclick={() => goto('/login/email')}
				>
					Log in / Sign up
				</Button>
			</div>
		</div>
	{/if}
</div>
