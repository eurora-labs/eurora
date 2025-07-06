<script lang="ts">
	import { Switch } from '@eurora/ui/components/switch/index';
	import { Label } from '@eurora/ui/components/label/index';
</script>

<div class="w-full h-full p-6 flex flex-col justify-start items-start gap-2">
	<h1 class="text-2xl font-bold">Telemetry</h1>
	<p class="text-sm text-muted-foreground">
		By enabling telemetry, you agree to share anonymous usage data with the Eurora team to help
		improve the app.
	</p>

	<div class="flex w-full items-start justify-start gap-2 py-2">
		<Switch />
		<Label>Send anonymous usage data</Label>
	</div>
</div>
