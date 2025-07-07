<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';

	import Hotkey from '$lib/components/Hotkey.svelte';
</script>

<div class="w-full h-full p-6 flex flex-col justify-between">
	<h1 class="text-2xl font-bold">Set up Eurora hotkey</h1>

	<Hotkey />

	<div class="flex justify-end">
		<Button variant="default" href="/">Continue</Button>
	</div>
</div>
