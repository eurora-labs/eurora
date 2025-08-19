<script lang="ts">
	import { Switch } from '@eurora/ui/components/switch/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { createTauRPCProxy, type HoverSettings } from '$lib/bindings/bindings';
	import { onMount } from 'svelte';

	const taurpc = createTauRPCProxy();

	let hoverSettings = $state<HoverSettings | null>(null);
	let hoverEnabled = $state(false);

	async function saveSettings() {
		await taurpc.settings.set_hover_settings({
			...hoverSettings,
			enabled: hoverEnabled,
		});
	}

	async function onHoverEnabledChange() {
		await taurpc.settings.set_hover_settings({
			...hoverSettings,
			enabled: hoverEnabled,
		});

		if (hoverEnabled) {
			taurpc.window.show_hover_window();
		} else {
			taurpc.window.hide_hover_window();
		}
	}

	onMount(() => {
		taurpc.settings.get_hover_settings().then((settings) => {
			hoverSettings = settings;
			hoverEnabled = hoverSettings?.enabled ?? false;
		});
	});
</script>

<div class="w-full h-full p-6 flex flex-col justify-start items-start gap-2">
	<h1 class="text-2xl font-bold">Hover Settings</h1>

	<div class="flex w-full items-start justify-start gap-2 py-2">
		<Switch bind:checked={hoverEnabled} onCheckedChange={onHoverEnabledChange} />
		<Label>Enable hover window</Label>
	</div>
</div>
