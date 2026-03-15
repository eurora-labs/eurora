<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/badge/index.js';
	import CheckCircle2Icon from '@lucide/svelte/icons/circle-check';
	import XCircleIcon from '@lucide/svelte/icons/circle-x';
	import CircleIcon from '@lucide/svelte/icons/circle';
	import { getTestResultsContext } from './test-results-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getTestResultsContext();
</script>

{#if ctx.summary}
	<div
		data-slot="test-results-summary"
		class={cn('flex items-center gap-3', className)}
		{...rest}
	>
		{#if children}
			{@render children()}
		{:else}
			<Badge
				class="gap-1 bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
				variant="secondary"
			>
				<CheckCircle2Icon class="size-3" />
				{ctx.summary.passed} passed
			</Badge>
			{#if ctx.summary.failed > 0}
				<Badge
					class="gap-1 bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
					variant="secondary"
				>
					<XCircleIcon class="size-3" />
					{ctx.summary.failed} failed
				</Badge>
			{/if}
			{#if ctx.summary.skipped > 0}
				<Badge
					class="gap-1 bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400"
					variant="secondary"
				>
					<CircleIcon class="size-3" />
					{ctx.summary.skipped} skipped
				</Badge>
			{/if}
		{/if}
	</div>
{/if}
