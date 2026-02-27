<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn, type WithElementRef } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLHeadingElement>> = $props();
</script>

<h2
	bind:this={ref}
	data-slot="feature-section-title"
	class={cn('text-3xl font-bold sm:text-4xl lg:text-5xl', className)}
	{...restProps}
>
	{@render children?.()}
</h2>
