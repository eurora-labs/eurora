<script lang="ts">
	import { BaseEdge, getBezierPath, type EdgeProps } from '@xyflow/svelte';

	let {
		id,
		sourceX,
		sourceY,
		targetX,
		targetY,
		sourcePosition,
		targetPosition,
		markerEnd,
		style,
	}: EdgeProps = $props();

	let [path] = $derived(
		getBezierPath({
			sourceX,
			sourceY,
			targetX,
			targetY,
			sourcePosition,
			targetPosition,
		}),
	);
</script>

{#if path}
	<BaseEdge data-slot="edge-animated" {id} {markerEnd} {path} {style} />
	<circle fill="var(--primary)" r="4">
		<animateMotion dur="2s" {path} repeatCount="indefinite" />
	</circle>
{/if}
