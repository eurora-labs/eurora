<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Collapsible } from '$lib/components/collapsible/index.js';
	import {
		TestSuiteState,
		setTestSuiteContext,
		type TestStatus,
	} from './test-results-context.svelte.js';

	interface Props {
		name: string;
		status: TestStatus;
		open?: boolean;
		class?: string;
		children?: Snippet;
	}

	let { class: className, name, status, open = $bindable(false), children }: Props = $props();

	let ctx = new TestSuiteState({ name, status });
	setTestSuiteContext(ctx);

	$effect(() => {
		ctx.name = name;
	});

	$effect(() => {
		ctx.status = status;
	});
</script>

<Collapsible data-slot="test-suite" class={cn('rounded-lg border', className)} bind:open>
	{@render children?.()}
</Collapsible>
