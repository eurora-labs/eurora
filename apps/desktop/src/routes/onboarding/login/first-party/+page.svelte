<script lang="ts">
	import { goto } from '$app/navigation';
	import { commands } from '$lib/bindings/specta.bindings.js';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import { open } from '@tauri-apps/plugin-shell';
	import { onMount, onDestroy } from 'svelte';

	const user = inject(USER_SERVICE);
	let intervalId: ReturnType<typeof setInterval> | null = null;

	async function openLogin() {
		const loginToken = await user.getLoginToken();
		await open(loginToken.url);

		intervalId = setInterval(async () => {
			const isLoginSuccess = await user.pollForLogin();
			if (!isLoginSuccess) {
				return;
			}
			clearInterval(intervalId!);
			// Best-effort focus — the user is mid-OAuth, swallow both
			// SystemError results and IPC rejections rather than blocking
			// the redirect on a window-focus hiccup.
			commands.systemFocusMainWindow().catch(() => {});
			if (user.emailVerified) {
				goto('/');
			} else {
				goto('/onboarding/login/verify-email?redirect=/');
			}
		}, 5000);
	}

	onMount(() => {
		openLogin().catch((err) => {
			console.error('Error opening login:', err);
		});
	});

	onDestroy(() => {
		if (intervalId) clearInterval(intervalId);
	});
</script>

<div class="relative flex h-full w-full flex-col px-8">
	<div class="flex flex-row justify-center items-center h-full w-full gap-4">
		<Spinner class="w-8 h-8" />
		<h1 class="text-4xl font-bold drop-shadow-lg">Waiting for you to log in...</h1>
	</div>
	<div class="mb-8">
		<Button variant="outline" size="default" onclick={() => goto('/onboarding')}>Cancel</Button>
	</div>
</div>
