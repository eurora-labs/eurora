<script lang="ts">
	import { Label } from '@eurora/ui/components/label/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { createTauRPCProxy, type GeneralSettings } from '$lib/bindings/bindings';
	import { onMount } from 'svelte';

	import FirstPartyLogin from '$lib/components/FirstPartyLogin.svelte';

	const tauRPC = createTauRPCProxy();

	let generalSettings = $state<GeneralSettings | null>(null);
	let autostartEnabled = $state(false);

	async function loadSettings() {
		generalSettings = await tauRPC.settings.get_general_settings();
		autostartEnabled = generalSettings?.autostart ?? false;
	}

	onMount(() => {
		loadSettings();
	});
</script>

<div class="flex flex-col p-4 gap-4">
	<h1 class="text-2xl font-semibold">General</h1>
	<div class="flex w-full items-start justify-start gap-2 py-2">
		<Switch checked={autostartEnabled} />
		<Label>Enable autostart</Label>
	</div>

	<FirstPartyLogin />

	<div class="flex flex-col gap-4">
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
