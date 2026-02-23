<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { CardContent } from '$lib/components/card/index.js';

	interface FlowNodeContentProps extends HTMLAttributes<HTMLDivElement> {
		class?: string;
		children?: Snippet;
	}

	let { class: className, children, ...restProps }: FlowNodeContentProps = $props();
</script>

<CardContent data-slot="flow-node-content" class={cn('p-3', className)} {...restProps}>
	{@render children?.()}
</CardContent>
