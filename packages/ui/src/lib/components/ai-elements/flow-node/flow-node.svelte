<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Card } from '$lib/components/card/index.js';
	import { Handle, Position } from '@xyflow/svelte';

	interface FlowNodeProps extends HTMLAttributes<HTMLDivElement> {
		handles: {
			target: boolean;
			source: boolean;
		};
		class?: string;
		children?: Snippet;
	}

	let { handles, class: className, children, ...restProps }: FlowNodeProps = $props();
</script>

<Card
	data-slot="flow-node"
	class={cn('node-container relative size-full h-auto w-sm gap-0 rounded-md p-0', className)}
	{...restProps}
>
	{#if handles.target}
		<Handle position={Position.Left} type="target" />
	{/if}
	{#if handles.source}
		<Handle position={Position.Right} type="source" />
	{/if}
	{@render children?.()}
</Card>
