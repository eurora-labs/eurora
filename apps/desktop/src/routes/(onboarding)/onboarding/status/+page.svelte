<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';
	import { onMount } from 'svelte';

	const tauRPC = createTauRPCProxy();

	let status = $state('Waiting...');

	async function getStatus() {
		status = await tauRPC.system.get_endpoint_status();
	}

	onMount(() => {
		getStatus();
	});
</script>

<div class="w-full h-screen mx-auto p-6 flex flex-col">
	<h1 class="text-2xl font-bold mb-8">Status</h1>
	<p>Checking connection to gRPC server...</p>

	<div class="flex justify-between items-end mt-auto pt-8">
		<Button variant="default" href="/onboarding">Disconnect</Button>
	</div>
</div>
