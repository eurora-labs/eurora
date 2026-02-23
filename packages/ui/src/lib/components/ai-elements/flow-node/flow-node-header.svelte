<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { CardHeader } from '$lib/components/card/index.js';

	interface FlowNodeHeaderProps extends HTMLAttributes<HTMLDivElement> {
		class?: string;
		children?: Snippet;
	}

	let { class: className, children, ...restProps }: FlowNodeHeaderProps = $props();
</script>

<CardHeader
	data-slot="flow-node-header"
	class={cn('gap-0.5 rounded-t-md border-b bg-secondary p-3!', className)}
	{...restProps}
>
	{@render children?.()}
</CardHeader>
