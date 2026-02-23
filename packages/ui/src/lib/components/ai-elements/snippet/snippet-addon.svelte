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
	data-slot="snippet-addon"
	class={cn('flex items-center border-r bg-muted px-3 text-muted-foreground', className)}
	{...restProps}
>
	{@render children?.()}
</div>
