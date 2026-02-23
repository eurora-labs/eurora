<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';

	let {
		class: className,
		count,
		label,
		icon,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		count?: number;
		label: string;
		icon?: Snippet;
		children?: Snippet;
	} = $props();
</script>

<span
	data-slot="queue-section-label"
	class={cn('flex items-center gap-2', className)}
	{...restProps}
>
	<ChevronDown class="size-4 transition-transform group-data-[state=closed]:-rotate-90" />
	{#if icon}
		{@render icon()}
	{/if}
	<span>
		{count} {label}
	</span>
</span>
