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
	data-slot="chain-of-thought-search-results"
	class={cn('flex flex-wrap items-center gap-2', className)}
	{...rest}
>
	{@render children?.()}
</div>
