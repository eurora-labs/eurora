<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn, type WithElementRef } from '$lib/utils.js';
	import { setStickToBottomContext } from './conversation-context.svelte.js';

	let {
		class: className,
		children,
		ref = $bindable(null),
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> & { children?: Snippet } = $props();

	setStickToBottomContext();
</script>

<div
	bind:this={ref}
	data-slot="conversation"
	class={cn('relative flex h-full flex-col overflow-hidden', className)}
	role="log"
	{...restProps}
>
	{@render children?.()}
</div>
