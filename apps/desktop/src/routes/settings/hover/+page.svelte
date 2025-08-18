<script lang="ts">
	import { Switch } from '@eurora/ui/components/switch/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { createTauRPCProxy } from '$lib/bindings/bindings';
	import { onMount } from 'svelte';

	const taurpc = createTauRPCProxy();

	let hoverEnabled = $state(false);

	async function loadSettings() {
		// hoverEnabled = await taurpc.settings.get_hover_enabled();
	}

	onMount(() => {
		loadSettings();
	});
</script>

<div class="w-full h-full p-6 flex flex-col justify-start items-start gap-2">
	<h1 class="text-2xl font-bold">Hover Settings</h1>

	<div class="flex w-full items-start justify-start gap-2 py-2">
		<Switch bind:checked={hoverEnabled} />
		<Label>Enable hover window</Label>
	</div>
</div>
