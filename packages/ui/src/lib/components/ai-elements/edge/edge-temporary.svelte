<script lang="ts">
	import { BaseEdge, getSmoothStepPath, type EdgeProps } from '@xyflow/svelte';

	let {
		id,
		sourceX,
		sourceY,
		targetX,
		targetY,
		sourcePosition,
		targetPosition,
	}: EdgeProps = $props();

	let edgePath = $derived(
		getSmoothStepPath({
			sourcePosition,
			sourceX,
			sourceY,
			targetPosition,
			targetX,
			targetY,
		}),
	);
</script>

<BaseEdge
	data-slot="edge-temporary"
	class="stroke-1 stroke-ring"
	{id}
	path={edgePath[0]}
	style="stroke-dasharray: 5, 5;"
/>
