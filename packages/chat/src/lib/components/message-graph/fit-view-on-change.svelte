<script lang="ts">
	import { useSvelteFlow } from '@xyflow/svelte';
	import { tick } from 'svelte';

	let { nodeCount }: { nodeCount: number } = $props();

	const { fitView } = useSvelteFlow();

	let prevNodeCount = $state(0);

	$effect(() => {
		if (nodeCount > 1 && nodeCount !== prevNodeCount) {
			prevNodeCount = nodeCount;
			tick().then(() => fitView({ padding: 0.2 }));
		}
	});
</script>
