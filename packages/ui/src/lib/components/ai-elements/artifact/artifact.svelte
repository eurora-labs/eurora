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
	data-slot="artifact"
	class={cn('flex flex-col overflow-hidden rounded-lg border bg-background shadow-sm', className)}
	{...restProps}
>
	{@render children?.()}
</div>
