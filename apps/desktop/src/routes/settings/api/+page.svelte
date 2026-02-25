<script lang="ts">
	import { type APISettings } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Checkbox } from '@eurora/ui/components/checkbox/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);

	let endpoint = $state('');
	let defaultEndpoint = $state('');
	let overrideEnabled = $state(false);

	async function save() {
		try {
			const settings: APISettings = {
				endpoint: overrideEnabled ? endpoint : defaultEndpoint,
				provider: null,
			};
			const result = await taurpc.settings.set_api_settings(settings);
			endpoint = result.endpoint;
			toast.success('Endpoint saved');
		} catch (error) {
			toast.error(`Failed to save: ${error}`);
		}
	}

	function onOverrideChange(checked: boolean) {
		overrideEnabled = checked;
		if (!checked) {
			endpoint = defaultEndpoint;
			save();
		}
	}

	onMount(async () => {
		const settings = await taurpc.settings.get_api_settings();
		defaultEndpoint = settings.endpoint;
		endpoint = settings.endpoint;
	});
</script>

<div class="flex flex-col gap-8">
	<div>
		<h1 class="text-lg font-semibold">API</h1>
		<p class="text-sm text-muted-foreground">Configure the backend endpoint.</p>
	</div>

	<section class="flex flex-col gap-4">
		<h2 class="text-sm font-medium text-muted-foreground">Endpoint</h2>
		<Separator />
		<Input
			placeholder="https://api.eurora-labs.com"
			bind:value={endpoint}
			disabled={!overrideEnabled}
			onchange={save}
		/>
		<div class="flex items-center gap-2">
			<Checkbox
				id="override-endpoint"
				checked={overrideEnabled}
				onCheckedChange={onOverrideChange}
			/>
			<Label for="override-endpoint" class="text-sm">Override endpoint</Label>
		</div>
	</section>
</div>
