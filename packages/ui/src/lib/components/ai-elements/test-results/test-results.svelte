<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import {
		TestResultsState,
		setTestResultsContext,
		type TestResultsSummary,
	} from './test-results-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		summary?: TestResultsSummary;
		children?: Snippet;
	}

	let { class: className, summary, children, ...rest }: Props = $props();

	const ctx = new TestResultsState({ summary: () => summary });
	setTestResultsContext(ctx);
</script>

<div data-slot="test-results" class={cn('rounded-lg border bg-background', className)} {...rest}>
	{@render children?.()}
</div>
