<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/badge/index.js';
	import Bot from '@lucide/svelte/icons/bot';

	let {
		class: className,
		name,
		model,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		name: string;
		model?: string;
		children?: Snippet;
	} = $props();
</script>

<div
	data-slot="agent-header"
	class={cn('flex w-full items-center justify-between gap-4 p-3', className)}
	{...restProps}
>
	<div class="flex items-center gap-2">
		<Bot class="size-4 text-muted-foreground" />
		<span class="font-medium">{name}</span>
		{#if model}
			<Badge class="font-mono text-xs" variant="secondary">{model}</Badge>
		{/if}
	</div>
	{@render children?.()}
</div>
