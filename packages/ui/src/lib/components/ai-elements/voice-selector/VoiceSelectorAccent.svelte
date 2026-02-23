<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { accentEmojiMap, type AccentValue } from './voice-selector-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		value?: AccentValue;
		children?: Snippet;
	}

	let { class: className, value, children, ...restProps }: Props = $props();

	let emoji = $derived(value ? accentEmojiMap[value] ?? null : null);
</script>

<span data-slot="voice-selector-accent" class={cn('text-muted-foreground text-xs', className)} {...restProps}>
	{#if children}
		{@render children()}
	{:else if emoji}
		{emoji}
	{/if}
</span>
