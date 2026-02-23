<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { StackTraceState, setStackTraceContext } from './stack-trace-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		trace: string;
		open?: boolean;
		defaultOpen?: boolean;
		onOpenChange?: (open: boolean) => void;
		onFilePathClick?: (filePath: string, line?: number, column?: number) => void;
		children?: Snippet;
	}

	let {
		class: className,
		trace,
		open = $bindable<boolean | undefined>(undefined),
		defaultOpen = false,
		onOpenChange,
		onFilePathClick,
		children,
		...rest
	}: Props = $props();

	let ctx = new StackTraceState({
		raw: trace,
		isOpen: open ?? defaultOpen,
		onFilePathClick,
	});
	setStackTraceContext(ctx);

	$effect(() => {
		ctx.raw = trace;
	});

	$effect(() => {
		if (open !== undefined) {
			ctx.isOpen = open;
		}
	});

	$effect(() => {
		ctx.onFilePathClick = onFilePathClick;
	});
</script>

<div
	data-slot="stack-trace"
	class={cn(
		'not-prose w-full overflow-hidden rounded-lg border bg-background font-mono text-sm',
		className,
	)}
	{...rest}
>
	{@render children?.()}
</div>
