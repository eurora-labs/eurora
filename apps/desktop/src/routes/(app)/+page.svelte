<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	let status = $state<'loading' | 'ready'>('loading');

	const taurpc = createTauRPCProxy();
	onMount(() => {
		taurpc.prompt
			.get_service_name()
			.then((name: string) => {
				if (name) {
					status = 'ready';
				}
			})
			.catch(() => {
				// goto('/onboarding');
			});
	});
</script>

<div class="w-full h-full p-6 flex flex-col justify-center items-center gap-4">
	{#if status === 'ready'}
		<h1 class="text-2xl font-bold">Eurora is ready!</h1>
		<p>Press PLACEHOLDER to open Eurora anywhere.</p>
		<div class="flex justify-start">You can change all the settings on your account page.</div>
	{:else}
		Checking...
	{/if}
</div>
