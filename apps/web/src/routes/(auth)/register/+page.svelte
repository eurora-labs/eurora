<script lang="ts">
	import SocialAuthButtons from '$lib/components/SocialAuthButtons.svelte';
	import { authService } from '$lib/services/auth-service';
	import { Provider } from '@eurora/shared/proto/auth_service_pb.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';

	let loading = $state(false);
	let submitError = $state<string | null>(null);

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

<div class="flex min-h-screen items-center justify-center px-4">
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
		</Card.Root>

		<div class="space-y-2 text-center">
			<p class="text-muted-foreground text-sm">
				Already have an account?
				<Button variant="link" href="/login" class="h-auto p-0 font-normal">Sign in</Button>
			</p>
			<p class="text-muted-foreground text-sm">
				Email &amp; password registration is coming soon.
			</p>
		</div>
	</div>
</div>
