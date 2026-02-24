<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLParagraphElement> & {
		children?: Snippet;
	} = $props();
</script>

<p
	data-slot="package-info-description"
	class={cn('mt-2 text-muted-foreground', className)}
	{...restProps}
>
	{@render children?.()}
</p>
