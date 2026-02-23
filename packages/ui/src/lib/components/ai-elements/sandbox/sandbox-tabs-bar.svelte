<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
	} = $props();
</script>

<div
	data-slot="sandbox-tabs-bar"
	class={cn('flex w-full items-center border-border border-t border-b', className)}
	{...restProps}
>
	{@render children?.()}
</div>
