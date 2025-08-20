<script lang="ts">
	import { Label } from '@eurora/ui/components/label/index';
	import type { LauncherSettings, Hotkey } from '$lib/bindings/bindings';
	import { default as HotkeyComponent } from '$lib/components/Hotkey.svelte';
	import { createTauRPCProxy } from '$lib/bindings/bindings';
	import { onMount } from 'svelte';

	const taurpc = createTauRPCProxy();

	let launcherSettings = $state<LauncherSettings | null>(null);
	let hotkey = $state<Hotkey | null>(null);

	async function onHotkeyChange(hotkey: Hotkey) {
		if (!hotkey) return;

		await taurpc.settings.set_launcher_settings({
			...launcherSettings,
			hotkey,
		});
	}

	onMount(() => {
		taurpc.settings.get_launcher_settings().then((settings) => {
			launcherSettings = settings;
			hotkey = settings.hotkey;
		});
	});
</script>

<div class="w-full h-full p-6 flex flex-col justify-start items-start gap-2">
	<h1 class="text-2xl font-bold">Launcher Settings</h1>

	<div class="flex w-full items-center justify-start gap-2 py-2">
		<Label>Current hotkey</Label>
		{#if hotkey}
			<HotkeyComponent variant="ghost" {hotkey} {onHotkeyChange} />
		{/if}
	</div>
</div>
