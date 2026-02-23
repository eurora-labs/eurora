<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { CardFooter } from '$lib/components/card/index.js';

	interface FlowNodeFooterProps extends HTMLAttributes<HTMLDivElement> {
		class?: string;
		children?: Snippet;
	}

	let { class: className, children, ...restProps }: FlowNodeFooterProps = $props();
</script>

<CardFooter
	data-slot="flow-node-footer"
	class={cn('rounded-b-md border-t bg-secondary p-3!', className)}
	{...restProps}
>
	{@render children?.()}
</CardFooter>
