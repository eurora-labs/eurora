<script lang="ts">
	import { untrack } from 'svelte';
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import {
		ChainOfThoughtState,
		setChainOfThoughtContext,
	} from './chain-of-thought-context.svelte.js';

	interface Props {
		class?: string;
		open?: boolean;
		defaultOpen?: boolean;
		onOpenChange?: (open: boolean) => void;
		isStreaming?: boolean;
		children?: Snippet;
		[key: string]: unknown;
	}

	let {
		class: className,
		open = $bindable<boolean | undefined>(undefined),
		defaultOpen = false,
		onOpenChange,
		isStreaming = false,
		children,
		...rest
	}: Props = $props();

	let internalOpen = $state(untrack(() => open ?? defaultOpen));
	const resolvedOpen = $derived(open ?? internalOpen);

	const ctx = new ChainOfThoughtState({
		isOpen: () => resolvedOpen,
		setOpen: (value) => setOpen(value),
		isStreaming: () => isStreaming,
	});
	setChainOfThoughtContext(ctx);

	export function setOpen(value: boolean) {
		internalOpen = value;
		if (open !== undefined) {
			open = value;
		}
		onOpenChange?.(value);
	}
</script>

<div data-slot="chain-of-thought" class={cn('not-prose w-full space-y-4', className)} {...rest}>
	{@render children?.()}
</div>
