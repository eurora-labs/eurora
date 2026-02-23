<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<div
	data-slot="inline-citation-carousel-header"
	class={cn('bg-secondary flex items-center justify-between gap-2 rounded-t-md p-2', className)}
	{...rest}
>
	{@render children?.()}
</div>
