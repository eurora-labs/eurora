<!-- <script lang="ts">
	import { Toaster } from '@eurora/ui/components/sonner/index';

	import { toast } from 'svelte-sonner';
	import Hotkey from '$lib/components/Hotkey.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';

	let hotkeyRef: Hotkey | null = $state(null);
	let loading = $state(false);

	async function handleSave() {
		loading = true;
		hotkeyRef?.saveHotkey();
		toast.success('Hotkey saved successfully');

		setTimeout(() => {
			loading = false;
		}, 1000);
	}
</script>

<Toaster />

<div class="w-full h-full p-6 flex flex-col justify-between">
	<h1 class="text-2xl font-bold">Set up Eurora hotkey</h1>

	<div class="flex w-full h-full items-center justify-center">
		<Hotkey bind:this={hotkeyRef} />
	</div>

	<div class="flex justify-end">
		<Button variant="default" onclick={handleSave} disabled={loading}>
			{#if loading}
				<Loader2Icon class="animate-spin mr-2" size={16} />
			{/if}
			Save
		</Button>
	</div>
</div> -->
