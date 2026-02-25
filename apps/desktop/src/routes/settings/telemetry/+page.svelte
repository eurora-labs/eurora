<script lang="ts">
	import { type TelemetrySettings } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Label } from '@eurora/ui/components/label/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import { onMount } from 'svelte';

	const taurpc = inject(TAURPC_SERVICE);

	let telemetrySettings = $state<TelemetrySettings | null>(null);
	let anonymousMetricsEnabled = $state(false);
	let anonymousErrorsEnabled = $state(false);
	let nonAnonymousMetricsEnabled = $state(false);

	async function loadSettings() {
		telemetrySettings = await taurpc.settings.get_telemetry_settings();
		anonymousMetricsEnabled = telemetrySettings?.anonymousMetrics ?? false;
		anonymousErrorsEnabled = telemetrySettings?.anonymousErrors ?? false;
		nonAnonymousMetricsEnabled = telemetrySettings?.nonAnonymousMetrics ?? false;
	}

	onMount(() => {
		loadSettings();
	});
</script>

<div class="flex flex-col gap-8">
	<div>
		<h1 class="text-lg font-semibold">Telemetry</h1>
		<p class="text-sm text-muted-foreground">
			Control what usage data is shared with the Eurora team.
		</p>
	</div>

	<section class="flex flex-col gap-4">
		<h2 class="text-sm font-medium text-muted-foreground">Data collection</h2>
		<Separator />
		<div class="flex items-center justify-between">
			<Label for="anon-metrics" class="text-sm">Anonymous metrics</Label>
			<Switch id="anon-metrics" bind:checked={anonymousMetricsEnabled} />
		</div>
		<div class="flex items-center justify-between">
			<Label for="anon-errors" class="text-sm">Anonymous errors</Label>
			<Switch id="anon-errors" bind:checked={anonymousErrorsEnabled} />
		</div>
		<div class="flex items-center justify-between">
			<Label for="non-anon-metrics" class="text-sm">Non-anonymous metrics</Label>
			<Switch id="non-anon-metrics" bind:checked={nonAnonymousMetricsEnabled} />
		</div>
	</section>
</div>
