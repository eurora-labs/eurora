<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HttpMethod } from './schema-display-context.svelte.js';
	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/badge/index.js';
	import { useSchemaDisplay } from './schema-display-context.svelte.js';

	let {
		class: className,
		children,
		...restProps
	}: {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	} = $props();

	const ctx = useSchemaDisplay();

	const methodStyles: Record<HttpMethod, string> = {
		DELETE: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
		GET: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
		PATCH: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
		POST: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
		PUT: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400',
	};
</script>

<Badge
	data-slot="schema-display-method"
	class={cn('font-mono text-xs', methodStyles[ctx.method], className)}
	variant="secondary"
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		{ctx.method}
	{/if}
</Badge>
