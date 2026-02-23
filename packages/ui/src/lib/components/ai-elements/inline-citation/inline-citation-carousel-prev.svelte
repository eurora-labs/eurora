<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import { getCarouselContext } from './inline-citation-context.svelte.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getCarouselContext();

	function handleClick() {
		ctx.prev();
	}
</script>

<button
	data-slot="inline-citation-carousel-prev"
	aria-label="Previous"
	class={cn('shrink-0', className)}
	onclick={handleClick}
	type="button"
	{...rest}
>
	{#if children}
		{@render children()}
	{:else}
		<ArrowLeftIcon class="text-muted-foreground size-4" />
	{/if}
</button>
