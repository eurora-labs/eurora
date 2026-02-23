<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { CardTitle } from '$lib/components/card/index.js';

	interface FlowNodeTitleProps extends HTMLAttributes<HTMLDivElement> {
		class?: string;
		children?: Snippet;
	}

	let { class: className, children, ...restProps }: FlowNodeTitleProps = $props();
</script>

<CardTitle data-slot="flow-node-title" class={cn(className)} {...restProps}>
	{@render children?.()}
</CardTitle>
