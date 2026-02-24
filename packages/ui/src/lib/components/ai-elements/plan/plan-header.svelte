<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { CardHeader } from '$lib/components/card/index.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
	} = $props();
</script>

<CardHeader
	data-slot="plan-header"
	class={cn('flex items-start justify-between', className)}
	{...restProps}
>
	{@render children?.()}
</CardHeader>
