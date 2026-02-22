<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import { open } from '@tauri-apps/plugin-shell';
	import { onDestroy, onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);
	const pricingUrl = 'https://www.eurora-labs.com/pricing';

	let interval: ReturnType<typeof setInterval> | undefined;

	async function openPricingAndPoll() {
		try {
			await open(pricingUrl);
		} catch {
			toast.error(`Could not open browser. Please visit: ${pricingUrl}`);
		}

		interval = setInterval(async () => {
			try {
				await taurpc.auth.refresh_session();
				const role = await taurpc.auth.get_role();
				if (role !== 'Tier1') return;
				clearInterval(interval);
				goto('/');
			} catch {
				// keep polling on transient errors
			}
		}, 5000);
	}

	onMount(() => {
		openPricingAndPoll();
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});
</script>

<div class="relative flex h-full w-full flex-col px-8">
	<div class="flex flex-row justify-center items-center h-full w-full gap-4">
		<Spinner class="w-8 h-8" />
		<h1 class="text-4xl font-bold drop-shadow-lg">Waiting for upgrade to complete...</h1>
	</div>
	<div class="mb-8">
		<Button variant="outline" size="default" onclick={() => goto('/no-access')}>Cancel</Button>
	</div>
</div>
