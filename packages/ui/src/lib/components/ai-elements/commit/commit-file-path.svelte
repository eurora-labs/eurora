<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		children?: Snippet;
	} = $props();
</script>

<span
	data-slot="commit-file-path"
	class={cn('truncate font-mono text-xs', className)}
	{...restProps}
>
	{@render children?.()}
</span>
