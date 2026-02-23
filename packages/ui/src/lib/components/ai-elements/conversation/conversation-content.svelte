<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn, type WithElementRef } from '$lib/utils.js';
	import { getStickToBottomContext } from './conversation-context.svelte.js';
	import { watch } from 'runed';

	let {
		class: className,
		children,
		ref = $bindable(null),
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> & { children?: Snippet } = $props();

	const context = getStickToBottomContext();
	let element: HTMLDivElement;

	watch(
		() => element,
		() => {
			if (element) {
				context.setElement(element);
				context.scrollToBottom('auto');
			}
		},
	);
</script>

<div
	bind:this={element}
	bind:this={ref}
	data-slot="conversation-content"
	class={cn('flex flex-1 flex-col gap-8 overflow-y-auto p-4', className)}
	{...restProps}
>
	{@render children?.()}
</div>
