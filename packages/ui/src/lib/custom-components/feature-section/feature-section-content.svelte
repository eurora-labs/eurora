<script lang="ts">
	import { cn, type WithElementRef } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();
</script>

<div
	bind:this={ref}
	data-slot="feature-section-content"
	class={cn(
		'flex flex-1 items-center justify-center overflow-hidden rounded-xl bg-muted shadow-lg',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</div>
