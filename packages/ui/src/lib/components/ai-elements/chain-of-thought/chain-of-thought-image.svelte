<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	interface Props {
		caption?: string;
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { caption, class: className, children, ...rest }: Props = $props();
</script>

<div data-slot="chain-of-thought-image" class={cn('mt-2 space-y-2', className)} {...rest}>
	<div
		class="bg-muted relative flex max-h-[22rem] items-center justify-center overflow-hidden rounded-lg p-3"
	>
		{@render children?.()}
	</div>
	{#if caption}
		<p class="text-muted-foreground text-xs">{caption}</p>
	{/if}
</div>
