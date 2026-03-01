<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import { open } from '@tauri-apps/plugin-shell';
	import { onMount, onDestroy } from 'svelte';

	const taurpc = inject(TAURPC_SERVICE);
	let intervalId: ReturnType<typeof setInterval> | null = null;

	async function openLogin() {
		const loginToken = await taurpc.auth.get_login_token();
		await open(loginToken.url);

		intervalId = setInterval(async () => {
			const isLoginSuccess = await taurpc.auth.poll_for_login();
			if (!isLoginSuccess) {
				return;
			}
			clearInterval(intervalId!);
			goto('/onboarding/login/first-party/browser-extension');
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
