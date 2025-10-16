<script lang="ts">
	import { Label } from '@eurora/ui/components/label/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { createTauRPCProxy, type GeneralSettings } from '$lib/bindings/bindings.js';
	import { onMount } from 'svelte';

	import FirstPartyLogin from '$lib/components/FirstPartyLogin.svelte';

	const tauRPC = createTauRPCProxy();

	let generalSettings = $state<GeneralSettings | null>(null);
	let autostartEnabled = $state(false);

	async function saveSettings() {
		await tauRPC.settings.set_general_settings({
			...generalSettings,
			autostart: autostartEnabled,
		});
	}

	onMount(() => {
		tauRPC.settings.get_general_settings().then((settings) => {
			generalSettings = settings;

			autostartEnabled = generalSettings.autostart;
		});
	});
</script>

<div class="flex flex-col p-4 gap-4">
	<h1 class="text-2xl font-semibold">General</h1>

	<FirstPartyLogin />

	<div class="flex flex-col gap-4">
		<div class="flex items-center gap-2">
			<Switch id="autostart" bind:checked={autostartEnabled} onCheckedChange={saveSettings} />
			<Label for="autostart">Enable autostart</Label>
		</div>
		<div class="flex items-center gap-2">
			<Label for="name">Name</Label>
			<Input id="name" disabled value="Eurora" />
		</div>

		<div class="flex items-center gap-2">
			<Label for="email">Email</Label>
			<Input id="email" disabled value="Eurora@eurora.ai" />
		</div>
	</div>
</div>
