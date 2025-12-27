<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { WithElementRef } from '$lib/utils.js';
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
	class={cn(
		'border bg-card text-card-foreground shadow-sm',
		'rounded-[40px] p-4 md:rounded-[120px] md:p-6',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</div>
