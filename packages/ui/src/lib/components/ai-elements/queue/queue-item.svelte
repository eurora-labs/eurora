<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLLIElement> & {
		children?: Snippet;
	} = $props();
</script>

<li
	data-slot="queue-item"
	class={cn(
		'group flex flex-col gap-1 rounded-md px-3 py-1 text-sm transition-colors hover:bg-muted',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</li>
