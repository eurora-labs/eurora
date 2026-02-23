<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { CardDescription } from '$lib/components/card/index.js';

	interface FlowNodeDescriptionProps extends HTMLAttributes<HTMLParagraphElement> {
		class?: string;
		children?: Snippet;
	}

	let { class: className, children, ...restProps }: FlowNodeDescriptionProps = $props();
</script>

<CardDescription data-slot="flow-node-description" class={cn(className)} {...restProps}>
	{@render children?.()}
</CardDescription>
