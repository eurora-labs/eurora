<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Separator } from '$lib/components/separator/index.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & { children?: Snippet } = $props();
</script>

<div
	data-slot="checkpoint"
	class={cn('flex items-center gap-0.5 overflow-hidden text-muted-foreground', className)}
	{...restProps}
>
	{@render children?.()}
	<Separator />
</div>
