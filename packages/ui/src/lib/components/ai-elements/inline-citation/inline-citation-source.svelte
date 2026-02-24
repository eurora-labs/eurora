<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	interface Props {
		title?: string;
		url?: string;
		description?: string;
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { title, url, description, class: className, children, ...rest }: Props = $props();
</script>

<div data-slot="inline-citation-source" class={cn('space-y-1', className)} {...rest}>
	{#if title}
		<h4 class="truncate font-medium leading-tight">{title}</h4>
	{/if}
	{#if url}
		<p class="text-muted-foreground truncate break-all text-xs">{url}</p>
	{/if}
	{#if description}
		<p class="text-muted-foreground line-clamp-3 leading-relaxed">{description}</p>
	{/if}
	{@render children?.()}
</div>
