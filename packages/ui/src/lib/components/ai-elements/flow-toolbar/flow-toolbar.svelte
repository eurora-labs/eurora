<script lang="ts">
	import type { Snippet } from 'svelte';
	import { NodeToolbar, Position } from '@xyflow/svelte';
	import { cn } from '$lib/utils.js';

	interface FlowToolbarProps {
		class?: string;
		children?: Snippet;
		[key: string]: any;
	}

	let { class: className, children, ...restProps }: FlowToolbarProps = $props();
</script>

<NodeToolbar
	data-slot="flow-toolbar"
	class={cn('flex items-center gap-1 rounded-sm border bg-background p-1.5', className)}
	position={Position.Bottom}
	{...restProps}
>
	{@render children?.()}
</NodeToolbar>
