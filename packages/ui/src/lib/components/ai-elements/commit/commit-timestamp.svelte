<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		class: className,
		date,
		children,
		...restProps
	}: HTMLAttributes<HTMLTimeElement> & {
		date: Date;
		children?: Snippet;
	} = $props();

	const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

	let formatted = $derived(
		rtf.format(Math.round((date.getTime() - Date.now()) / (1000 * 60 * 60 * 24)), 'day'),
	);
</script>

<time
	data-slot="commit-timestamp"
	class={cn('text-xs', className)}
	datetime={date.toISOString()}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		{formatted}
	{/if}
</time>
