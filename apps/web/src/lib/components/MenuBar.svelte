<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import type { Snippet } from 'svelte';

	let {
		actions,
	}: {
		actions?: Snippet;
	} = $props();
</script>

<div class="bg-transparent z-0 flex items-center justify-between px-6 py-4 mt-2">
	<div class="flex items-center gap-2">
		<Button variant="link" href="/" class="decoration-transparent">
			<EuroraLogo style="width: 2rem; height: 2rem;" />
			<span class="text-lg text-primary-foreground font-bold">Eurora</span>
		</Button>
	</div>

	{#if actions}
		<div class="flex items-center gap-4">
			{@render actions()}
		</div>
	{/if}
</div>
