<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { HoverCardTrigger } from '$lib/components/hover-card/index.js';
	import { Badge } from '$lib/components/badge/index.js';

	interface Props {
		sources: string[];
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { sources, class: className, children, ...rest }: Props = $props();

	let hostname = $derived(() => {
		try {
			return sources[0] ? new URL(sources[0]).hostname : null;
		} catch {
			return null;
		}
	});
</script>

<HoverCardTrigger data-slot="inline-citation-card-trigger">
	{#if children}
		{@render children()}
	{:else}
		<Badge class={cn('ml-1 rounded-full', className)} variant="secondary" {...rest}>
			{#if hostname()}
				{hostname()}
				{#if sources.length > 1}
					{' '}+{sources.length - 1}
				{/if}
			{:else}
				unknown
			{/if}
		</Badge>
	{/if}
</HoverCardTrigger>
