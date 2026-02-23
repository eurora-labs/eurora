<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLHeadingElement> & {
		children?: Snippet;
	} = $props();
</script>

<h3
	data-slot="prompt-input-tab-label"
	class={cn('mb-2 px-3 font-medium text-muted-foreground text-xs', className)}
	{...restProps}
>
	{@render children?.()}
</h3>
