<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';

	let settingHotkey = $state(false);

	async function setHotkey() {
		settingHotkey = true;
	}
</script>

<div class="w-full h-full p-6 flex flex-col justify-between items-center">
	<h1 class="text-2xl font-bold">Set up hotkey</h1>
	<div class="flex flex-col justify-center items-center gap-4">
		<h2 class="text-lg font-semibold w-full">Current hotkey: CTRL + Space</h2>
		<Button disabled={settingHotkey} onclick={setHotkey} variant="default">
			{#if settingHotkey}
				<Loader2Icon class="animate-spin" />
				Press any key...
			{:else}
				Set hotkey
			{/if}
		</Button>
	</div>

	<div class="flex">
		<Button variant="default" href="/onboarding">Cancel</Button>
	</div>
</div>
