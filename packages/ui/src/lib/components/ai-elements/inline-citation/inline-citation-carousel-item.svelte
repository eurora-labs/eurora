<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { getCarouselContext } from './inline-citation-context.svelte.js';

	interface Props {
		index: number;
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { index, class: className, children, ...rest }: Props = $props();

	let ctx = getCarouselContext();

	let isActive = $derived(ctx.currentIndex === index);
</script>

{#if isActive}
	<div
		data-slot="inline-citation-carousel-item"
		class={cn('w-full space-y-2 p-4', className)}
		{...rest}
	>
		{@render children?.()}
	</div>
{/if}
