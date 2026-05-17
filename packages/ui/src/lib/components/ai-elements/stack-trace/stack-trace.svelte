<script lang="ts">
	import { untrack } from 'svelte';
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

	let internalOpen = $state(untrack(() => open ?? defaultOpen));

	const ctx = new StackTraceState({
		raw: () => trace,
		isOpen: () => open ?? internalOpen,
		setOpen: (value) => {
			internalOpen = value;
			if (open !== undefined) {
				open = value;
			}
			onOpenChange?.(value);
		},
		onFilePathClick: () => onFilePathClick,
	});
	setStackTraceContext(ctx);
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
