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
	data-slot="prompt-input-tab-item"
	class={cn('flex items-center gap-2 px-3 py-2 text-xs hover:bg-accent', className)}
	{...restProps}
>
	{@render children?.()}
</div>
