<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button, type ButtonVariant, type ButtonSize } from '$lib/components/button/index.js';
	import XIcon from '@lucide/svelte/icons/x';

	let {
		class: className,
		children,
		size = 'sm' as ButtonSize,
		variant = 'ghost' as ButtonVariant,
		...restProps
	}: {
		class?: string;
		children?: Snippet;
		size?: ButtonSize;
		variant?: ButtonVariant;
		[key: string]: any;
	} = $props();
</script>

<Button
	data-slot="artifact-close"
	class={cn('size-8 p-0 text-muted-foreground hover:text-foreground', className)}
	{size}
	type="button"
	{variant}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<XIcon class="size-4" />
	{/if}
	<span class="sr-only">Close</span>
</Button>
