<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import CheckCircle2Icon from '@lucide/svelte/icons/circle-check';
	import XCircleIcon from '@lucide/svelte/icons/circle-x';
	import CircleIcon from '@lucide/svelte/icons/circle';
	import CircleDotIcon from '@lucide/svelte/icons/circle-dot';
	import { getTestCaseContext, type TestStatus } from './test-results-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getTestCaseContext();

	const statusStyles: Record<TestStatus, string> = {
		passed: 'text-green-600 dark:text-green-400',
		failed: 'text-red-600 dark:text-red-400',
		skipped: 'text-yellow-600 dark:text-yellow-400',
		running: 'text-blue-600 dark:text-blue-400',
	};
</script>

<span
	data-slot="test-case-status"
	class={cn('shrink-0', statusStyles[ctx.status], className)}
	{...rest}
>
	{#if children}
		{@render children()}
	{:else if ctx.status === 'passed'}
		<CheckCircle2Icon class="size-4" />
	{:else if ctx.status === 'failed'}
		<XCircleIcon class="size-4" />
	{:else if ctx.status === 'running'}
		<CircleDotIcon class="size-4 animate-pulse" />
	{:else}
		<CircleIcon class="size-4" />
	{/if}
</span>
