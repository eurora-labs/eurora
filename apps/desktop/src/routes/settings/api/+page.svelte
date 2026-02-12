<script lang="ts">
	import { type ApiSettings } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Checkbox } from '@eurora/ui/components/checkbox/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { onMount } from 'svelte';

	const taurpc = inject(TAURPC_SERVICE);

	let apiSettings = $state<ApiSettings | null>(null);
	let customEndpoint = $state(false);
	let endpointValue = $state('');

	async function saveSettings() {
		if (!apiSettings) return;
		await taurpc.settings.set_api_settings({
			...apiSettings,
			endpoint: customEndpoint ? endpointValue : apiSettings.endpoint,
		});
	}

	function onCustomEndpointChange(checked: boolean) {
		customEndpoint = checked;
		if (!checked && apiSettings) {
			// Reset to current endpoint when unchecking
			endpointValue = '';
			saveSettings();
		}
	}

	onMount(() => {
		taurpc.settings.get_api_settings().then((settings) => {
			apiSettings = settings;
		});
	});
</script>

<div class="flex flex-col p-4 gap-4">
	<h1 class="text-2xl font-semibold">Api Settings</h1>

	<div class="flex flex-col gap-4">
		<div class="flex items-center gap-2">
			<Checkbox
				id="custom-endpoint"
				bind:checked={customEndpoint}
				onCheckedChange={onCustomEndpointChange}
			/>
			<Label for="custom-endpoint">Custom endpoint</Label>
		</div>

		{#if customEndpoint}
			<div class="flex flex-col gap-2">
				<Label for="endpoint">Endpoint</Label>
				<Input
					id="endpoint"
					placeholder={apiSettings?.endpoint ?? ''}
					bind:value={endpointValue}
					onblur={saveSettings}
				/>
			</div>
		{/if}
	</div>
</div>
