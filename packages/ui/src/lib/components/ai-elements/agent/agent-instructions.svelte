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

<div data-slot="agent-instructions" class={cn('space-y-2', className)} {...restProps}>
	<span class="font-medium text-muted-foreground text-sm">Instructions</span>
	<div class="rounded-md bg-muted/50 p-3 text-muted-foreground text-sm">
		{@render children?.()}
	</div>
</div>
