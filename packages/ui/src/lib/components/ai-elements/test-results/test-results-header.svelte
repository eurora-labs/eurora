<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<div
	data-slot="test-results-header"
	class={cn('flex items-center justify-between border-b px-4 py-3', className)}
	{...rest}
>
	{@render children?.()}
</div>
