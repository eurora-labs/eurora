<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { default as HotkeyComponent } from '$lib/components/Hotkey.svelte';
	import type { Hotkey, LauncherSettings } from '$lib/bindings/bindings.js';
	import { goto } from '$app/navigation';
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';
	import { onMount } from 'svelte';

	let taurpc = createTauRPCProxy();

	let launcherSettings = $state<LauncherSettings | undefined>(undefined);
	let hotkey = $state<Hotkey | undefined>(undefined);

	onMount(() => {
		taurpc.settings.get_launcher_settings().then((settings) => {
			launcherSettings = settings;
			hotkey = settings.hotkey;
		});
	});

	async function onHotkeyChange(key: Hotkey) {
		await taurpc.settings.set_launcher_settings({ ...launcherSettings, hotkey: key });
		goto('/onboarding/hotkey/trial');
	}

	function handleSkip() {
		goto('/');
	}
</script>

<div class="w-full h-full p-6 flex flex-col justify-between">
	<h1 class="text-2xl font-bold">Set up Eurora hotkey</h1>

	{#if hotkey}
		<div class="w-full items-center flex flex-col">
			<HotkeyComponent variant="default" {hotkey} {onHotkeyChange} />
		</div>
	{/if}

	<div class="flex justify-end">
		<Button variant="secondary" onclick={handleSkip}>Skip</Button>
	</div>
</div>
