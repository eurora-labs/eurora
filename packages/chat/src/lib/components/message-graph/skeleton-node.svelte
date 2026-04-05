<script lang="ts">
	import * as FlowNode from '@eurora/ui/components/ai-elements/flow-node/index';
	import { Skeleton } from '@eurora/ui/components/skeleton/index';

	export interface SkeletonNodeData {
		role: 'user' | 'assistant';
		handles: { target: boolean; source: boolean };
	}

	let { data }: { data: SkeletonNodeData } = $props();

	const roleLabel = $derived(data.role === 'user' ? 'User' : 'Assistant');
</script>

<div class="skeleton-node flex flex-col items-center gap-2">
	<FlowNode.Root handles={data.handles}>
		<FlowNode.Header>
			<FlowNode.Title>{roleLabel}</FlowNode.Title>
		</FlowNode.Header>
		<FlowNode.Content>
			<div class="flex flex-col gap-2">
				<Skeleton class="shimmer bg-muted h-3 w-full" />
				<Skeleton class="shimmer bg-muted h-3 w-3/4" />
				{#if data.role === 'assistant'}
					<Skeleton class="shimmer bg-muted h-3 w-5/6" />
				{/if}
			</div>
		</FlowNode.Content>
	</FlowNode.Root>
</div>

<style>
	.skeleton-node :global(.shimmer) {
		background-image: linear-gradient(
			110deg,
			transparent 25%,
			var(--muted-foreground) 37%,
			transparent 63%
		);
	}
</style>
