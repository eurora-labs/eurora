<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn, type WithElementRef } from '$lib/utils.js';

	let {
		class: className,
		title = 'No messages yet',
		description = 'Start a conversation to see messages here',
		icon,
		children,
		ref = $bindable(null),
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> & {
		title?: string;
		description?: string;
		icon?: Snippet;
		children?: Snippet;
	} = $props();
</script>

<div
	bind:this={ref}
	data-slot="conversation-empty-state"
	class={cn(
		'flex size-full flex-col items-center justify-center gap-3 p-8 text-center',
		className,
	)}
	{...restProps}
>
	{#if children}
		{@render children?.()}
	{:else}
		{#if icon}
			<div class="text-muted-foreground">
				{@render icon()}
			</div>
		{/if}
		<div class="space-y-1">
			<h3 class="font-medium">{title}</h3>
			{#if description}
				<p class="text-muted-foreground">{description}</p>
			{/if}
		</div>
	{/if}
</div>
