<script lang="ts">
	import { Switch } from '@eurora/ui/components/switch/index';
	import { Label } from '@eurora/ui/components/label/index';

	import { createTauRPCProxy, type TelemetrySettings } from '$lib/bindings/bindings.js';
	import { onMount } from 'svelte';

	const taurpc = createTauRPCProxy();

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

<div class="w-full h-full p-6 flex flex-col justify-start items-start gap-2">
	<h1 class="text-2xl font-bold">Telemetry</h1>
	<p class="text-sm text-muted-foreground">
		By enabling telemetry, you agree to share anonymous usage data with the Eurora team to help
		improve the app.
	</p>

	<div class="flex w-full items-start justify-start gap-2 py-2">
		<Switch checked={anonymousMetricsEnabled} />
		<Label>Send anonymous metrics</Label>
	</div>

	<div class="flex w-full items-start justify-start gap-2 py-2">
		<Switch checked={anonymousErrorsEnabled} />
		<Label>Send anonymous errors</Label>
	</div>
	<div class="flex w-full items-start justify-start gap-2 py-2">
		<Switch checked={nonAnonymousMetricsEnabled} />
		<Label>Send non-anonymous metrics</Label>
	</div>
</div>
