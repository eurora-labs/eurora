<script lang="ts">
	import { useSvelteFlow } from '@xyflow/svelte';
	import { tick } from 'svelte';

	let { nodeCount }: { nodeCount: number } = $props();

	const { fitView } = useSvelteFlow();

	$effect(() => {
		if (nodeCount > 1) {
			tick().then(() => fitView({ padding: 0.2 }));
		}
	});
</script>
