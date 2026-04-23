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
	class={cn('typo-body mt-6 max-w-2xl text-muted-foreground', className)}
	{...restProps}
>
	{@render children?.()}
</p>
