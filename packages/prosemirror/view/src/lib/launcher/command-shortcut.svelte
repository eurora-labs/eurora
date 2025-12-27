<script lang="ts">
	import { cn } from '$lib/utils';
	import type { WithElementRef } from '$lib/utils';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLSpanElement>> = $props();
</script>

<span
	class={cn('text-muted-foreground ml-auto text-xs tracking-widest', className)}
	{...restProps}
	bind:this={ref}
>
	{@render children?.()}
</span>
