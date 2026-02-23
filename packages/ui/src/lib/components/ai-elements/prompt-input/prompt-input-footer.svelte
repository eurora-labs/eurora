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
	data-slot="prompt-input-footer"
	class={cn('flex items-center justify-between gap-1', className)}
	{...restProps}
>
	{@render children?.()}
</div>
