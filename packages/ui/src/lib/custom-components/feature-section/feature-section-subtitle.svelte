<script lang="ts">
	import { cn, type WithElementRef } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLParagraphElement>> = $props();
</script>

<p
	bind:this={ref}
	data-slot="feature-section-subtitle"
	class={cn('mt-6 text-xl text-muted-foreground sm:text-2xl', className)}
	{...restProps}
>
	{@render children?.()}
</p>
