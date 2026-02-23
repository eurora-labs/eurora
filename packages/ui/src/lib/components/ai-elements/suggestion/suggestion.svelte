<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button, type ButtonVariant, type ButtonSize } from '$lib/components/button/index.js';

	let {
		suggestion,
		onclick,
		variant = 'outline',
		size = 'sm',
		class: className,
		children,
		...restProps
	}: {
		suggestion: string;
		onclick?: (suggestion: string) => void;
		variant?: ButtonVariant;
		size?: ButtonSize;
		class?: string;
		children?: Snippet;
	} = $props();

	function handleClick() {
		onclick?.(suggestion);
	}
</script>

<Button
	data-slot="suggestion"
	class={cn('cursor-pointer rounded-full px-4', className)}
	onclick={handleClick}
	{size}
	type="button"
	{variant}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		{suggestion}
	{/if}
</Button>
