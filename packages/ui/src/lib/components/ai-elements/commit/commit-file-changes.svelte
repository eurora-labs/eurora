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
	data-slot="commit-file-changes"
	class={cn('flex shrink-0 items-center gap-1 font-mono text-xs', className)}
	{...restProps}
>
	{@render children?.()}
</div>
