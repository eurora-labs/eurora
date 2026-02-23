<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { CardAction } from '$lib/components/card/index.js';

	interface FlowNodeActionProps extends HTMLAttributes<HTMLDivElement> {
		class?: string;
		children?: Snippet;
	}

	let { class: className, children, ...restProps }: FlowNodeActionProps = $props();
</script>

<CardAction data-slot="flow-node-action" class={cn(className)} {...restProps}>
	{@render children?.()}
</CardAction>
