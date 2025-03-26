<script lang="ts">
	import type { WithElementRef } from 'bits-ui';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '@eurora/ui/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();
</script>

<div
	bind:this={ref}
	class={cn(
		'bg-card text-card-foreground rounded-[120px] border pb-6 pl-6 pr-6 pt-6 shadow-sm',
		className
	)}
	{...restProps}
>
	{@render children?.()}
</div>
