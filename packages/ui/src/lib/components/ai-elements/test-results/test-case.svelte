<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import {
		TestCaseState,
		setTestCaseContext,
		type TestStatus,
	} from './test-results-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		name: string;
		status: TestStatus;
		duration?: number;
		children?: Snippet;
	}

	let { class: className, name, status, duration, children, ...rest }: Props = $props();

	let ctx = new TestCaseState({ name, status, duration });
	setTestCaseContext(ctx);

	$effect(() => {
		ctx.name = name;
	});

	$effect(() => {
		ctx.status = status;
	});

	$effect(() => {
		ctx.duration = duration;
	});
</script>

<div
	data-slot="test-case"
	class={cn('flex items-center gap-2 px-4 py-2 text-sm', className)}
	{...rest}
>
	{@render children?.()}
</div>
