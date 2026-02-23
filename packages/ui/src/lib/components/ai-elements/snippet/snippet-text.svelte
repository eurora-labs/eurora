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
	data-slot="snippet-text"
	class={cn('pl-2 font-normal text-muted-foreground', className)}
	{...restProps}
>
	{@render children?.()}
</span>
