<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { getCarouselContext } from './inline-citation-context.svelte.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getCarouselContext();

	let display = $derived(`${ctx.currentIndex + 1}/${ctx.total}`);
</script>

<div
	data-slot="inline-citation-carousel-index"
	class={cn(
		'text-muted-foreground flex flex-1 items-center justify-end px-3 py-1 text-xs',
		className,
	)}
	{...rest}
>
	{#if children}
		{@render children()}
	{:else}
		{display}
	{/if}
</div>
