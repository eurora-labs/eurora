<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { ChangeType } from './package-info-context.svelte.js';
	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/badge/index.js';
	import ArrowRight from '@lucide/svelte/icons/arrow-right';
	import Plus from '@lucide/svelte/icons/plus';
	import Minus from '@lucide/svelte/icons/minus';
	import { usePackageInfo } from './package-info-context.svelte.js';

	let {
		class: className,
		children,
		...restProps
	}: {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	} = $props();

	const ctx = usePackageInfo();

	const changeTypeStyles: Record<ChangeType, string> = {
		added: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
		major: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
		minor: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
		patch: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
		removed: 'bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400',
	};
</script>

{#if ctx.changeType}
	<Badge
		data-slot="package-info-change-type"
		class={cn('gap-1 text-xs capitalize', changeTypeStyles[ctx.changeType], className)}
		variant="secondary"
		{...restProps}
	>
		{#if ctx.changeType === 'added'}
			<Plus class="size-3" />
		{:else if ctx.changeType === 'removed'}
			<Minus class="size-3" />
		{:else}
			<ArrowRight class="size-3" />
		{/if}
		{#if children}
			{@render children()}
		{:else}
			{ctx.changeType}
		{/if}
	</Badge>
{/if}
