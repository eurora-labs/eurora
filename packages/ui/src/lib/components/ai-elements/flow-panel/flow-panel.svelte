<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Panel } from '@xyflow/svelte';
	import { cn } from '$lib/utils.js';

	interface FlowPanelProps {
		class?: string;
		children?: Snippet;
		[key: string]: any;
	}

	let { class: className, children, ...restProps }: FlowPanelProps = $props();
</script>

<Panel
	data-slot="flow-panel"
	class={cn('m-4 overflow-hidden rounded-md border bg-card p-1', className)}
	{...restProps}
>
	{@render children?.()}
</Panel>
