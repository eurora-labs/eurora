<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import { getCarouselContext } from './inline-citation-context.svelte.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getCarouselContext();

	function handleClick() {
		ctx.next();
	}
</script>

<button
	data-slot="inline-citation-carousel-next"
	aria-label="Next"
	class={cn('shrink-0', className)}
	onclick={handleClick}
	type="button"
	{...rest}
>
	{#if children}
		{@render children()}
	{:else}
		<ArrowRightIcon class="text-muted-foreground size-4" />
	{/if}
</button>
