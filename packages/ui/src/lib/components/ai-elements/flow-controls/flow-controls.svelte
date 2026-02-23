<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Controls } from '@xyflow/svelte';
	import { cn } from '$lib/utils.js';

	interface FlowControlsProps {
		class?: string;
		children?: Snippet;
		[key: string]: any;
	}

	let { class: className, children, ...restProps }: FlowControlsProps = $props();
</script>

<Controls
	data-slot="flow-controls"
	class={cn(
		'gap-px overflow-hidden rounded-md border bg-card p-1 shadow-none!',
		'[&>button]:rounded-md [&>button]:border-none! [&>button]:bg-transparent! [&>button]:hover:bg-secondary!',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</Controls>
