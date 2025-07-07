<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import Hotkey from '$lib/components/Hotkey.svelte';
	import { goto } from '$app/navigation';

	let hotkeyRef: Hotkey | null = $state(null);

	function handleContinue() {
		if (hotkeyRef) {
			hotkeyRef.saveHotkey();
		}
		goto('/');
	}
</script>

<div class="w-full h-full p-6 flex flex-col justify-between">
	<h1 class="text-2xl font-bold">Set up Eurora hotkey</h1>

	<Hotkey bind:this={hotkeyRef} />

	<div class="flex justify-end">
		<Button variant="default" onclick={handleContinue}>Continue</Button>
	</div>
</div>
