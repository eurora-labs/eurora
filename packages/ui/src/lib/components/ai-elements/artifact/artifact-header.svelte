<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & { children?: Snippet } = $props();
</script>

<div
	data-slot="artifact-header"
	class={cn('flex items-center justify-between border-b bg-muted/50 px-4 py-3', className)}
	{...restProps}
>
	{@render children?.()}
</div>
