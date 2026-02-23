<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { CarouselState, setCarouselContext } from './inline-citation-context.svelte.js';

	interface Props {
		class?: string;
		total?: number;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, total = 0, children, ...rest }: Props = $props();

	let ctx = new CarouselState({ currentIndex: 0, total });

	setCarouselContext(ctx);

	$effect(() => {
		ctx.total = total;
	});
</script>

<div data-slot="inline-citation-carousel" class={cn('w-full', className)} {...rest}>
	{@render children?.()}
</div>
