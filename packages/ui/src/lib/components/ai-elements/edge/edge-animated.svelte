<script lang="ts">
	import { BaseEdge, getBezierPath, useSvelteFlow, type EdgeProps } from '@xyflow/svelte';
	import { getEdgeParams } from './edge-utils.js';

	let { id, source, target, markerEnd, style }: EdgeProps = $props();

	const { getInternalNode } = useSvelteFlow();

	let edgeParams = $derived.by(() => {
		const sourceNode = getInternalNode(source);
		const targetNode = getInternalNode(target);

		if (!(sourceNode && targetNode)) {
			return null;
		}

		return getEdgeParams(sourceNode, targetNode);
	});

	let edgePath = $derived.by(() => {
		if (!edgeParams) return null;

		return getBezierPath({
			sourcePosition: edgeParams.sourcePos,
			sourceX: edgeParams.sx,
			sourceY: edgeParams.sy,
			targetPosition: edgeParams.targetPos,
			targetX: edgeParams.tx,
			targetY: edgeParams.ty,
		});
	});
</script>

{#if edgePath}
	<BaseEdge data-slot="edge-animated" {id} {markerEnd} path={edgePath[0]} {style} />
	<circle fill="var(--primary)" r="4">
		<animateMotion dur="2s" path={edgePath[0]} repeatCount="indefinite" />
	</circle>
{/if}
