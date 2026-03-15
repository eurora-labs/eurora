<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { CollapsibleTrigger } from '$lib/components/collapsible/index.js';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import CheckCircle2Icon from '@lucide/svelte/icons/circle-check';
	import XCircleIcon from '@lucide/svelte/icons/circle-x';
	import CircleIcon from '@lucide/svelte/icons/circle';
	import CircleDotIcon from '@lucide/svelte/icons/circle-dot';
	import { getTestSuiteContext, type TestStatus } from './test-results-context.svelte.js';

	interface Props {
		class?: string;
		children?: Snippet;
	}

	let { class: className, children }: Props = $props();

	let ctx = getTestSuiteContext();

	const statusStyles: Record<TestStatus, string> = {
		passed: 'text-green-600 dark:text-green-400',
		failed: 'text-red-600 dark:text-red-400',
		skipped: 'text-yellow-600 dark:text-yellow-400',
		running: 'text-blue-600 dark:text-blue-400',
	};
</script>

<CollapsibleTrigger
	data-slot="test-suite-header"
	class={cn(
		'group flex w-full items-center gap-2 px-4 py-3 text-left transition-colors hover:bg-muted/50',
		className,
	)}
>
	{#if children}
		{@render children()}
	{:else}
		<ChevronRightIcon
			class="size-4 shrink-0 text-muted-foreground transition-transform group-data-[state=open]:rotate-90"
		/>
		<span class={cn('shrink-0', statusStyles[ctx.status])}>
			{#if ctx.status === 'passed'}
				<CheckCircle2Icon class="size-4" />
			{:else if ctx.status === 'failed'}
				<XCircleIcon class="size-4" />
			{:else if ctx.status === 'running'}
				<CircleDotIcon class="size-4 animate-pulse" />
			{:else}
				<CircleIcon class="size-4" />
			{/if}
		</span>
		<span class="font-medium text-sm">{ctx.name}</span>
	{/if}
</CollapsibleTrigger>
