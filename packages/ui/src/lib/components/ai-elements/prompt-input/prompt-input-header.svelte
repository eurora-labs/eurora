<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { InputGroupAddon } from '$lib/components/input-group/index.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
	} = $props();
</script>

<InputGroupAddon
	data-slot="prompt-input-header"
	align="block-end"
	class={cn('order-first flex-wrap gap-1', className)}
	{...restProps}
>
	{@render children?.()}
</InputGroupAddon>
