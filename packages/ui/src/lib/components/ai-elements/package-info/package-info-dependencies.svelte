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

<div data-slot="package-info-dependencies" class={cn('space-y-2', className)} {...restProps}>
	<span class="font-medium text-muted-foreground text-xs uppercase tracking-wide">
		Dependencies
	</span>
	<div class="space-y-1">
		{@render children?.()}
	</div>
</div>
