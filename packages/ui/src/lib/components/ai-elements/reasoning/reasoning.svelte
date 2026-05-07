<script lang="ts">
	import { untrack } from 'svelte';
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Collapsible } from '$lib/components/collapsible/index.js';
	import { ReasoningState, setReasoningContext } from './reasoning-context.svelte.js';

	interface Props {
		class?: string;
		isStreaming?: boolean;
		open?: boolean;
		defaultOpen?: boolean;
		onOpenChange?: (open: boolean) => void;
		duration?: number;
		children?: Snippet;
		[key: string]: unknown;
	}

	let {
		class: className,
		isStreaming = false,
		open = $bindable<boolean | undefined>(undefined),
		defaultOpen,
		onOpenChange,
		duration: durationProp,
		children,
		...rest
	}: Props = $props();

	const AUTO_CLOSE_DELAY = 1000;
	const MS_IN_S = 1000;

	const { resolvedDefaultOpen, isExplicitlyClosed } = untrack(() => ({
		resolvedDefaultOpen: defaultOpen ?? isStreaming,
		isExplicitlyClosed: defaultOpen === false,
	}));

	let isOpen = $state(untrack(() => open ?? resolvedDefaultOpen));
	let hasEverStreamed = $state(untrack(() => isStreaming));
	let hasAutoClosed = $state(false);
	let startTime = $state<number | null>(null);
	let measuredDuration = $state<number | undefined>(undefined);

	const ctx = new ReasoningState({
		isStreaming: () => isStreaming,
		isOpen: () => isOpen,
		duration: () => durationProp ?? measuredDuration,
	});
	setReasoningContext(ctx);

	$effect(() => {
		if (isStreaming) {
			hasEverStreamed = true;
			if (startTime === null) {
				startTime = Date.now();
			}
		} else if (startTime !== null) {
			measuredDuration = Math.ceil((Date.now() - startTime) / MS_IN_S);
			startTime = null;
		}
	});

	$effect(() => {
		if (isStreaming && !isOpen && !isExplicitlyClosed) {
			setOpen(true);
		}
	});

	$effect(() => {
		if (hasEverStreamed && !isStreaming && isOpen && !hasAutoClosed) {
			const timer = setTimeout(() => {
				setOpen(false);
				hasAutoClosed = true;
			}, AUTO_CLOSE_DELAY);

			return () => clearTimeout(timer);
		}
	});

	function setOpen(value: boolean) {
		isOpen = value;
		if (open !== undefined) {
			open = value;
		}
		onOpenChange?.(value);
	}

	function handleOpenChange(value: boolean) {
		setOpen(value);
	}
</script>

<Collapsible
	data-slot="reasoning"
	class={cn('not-prose mb-4', className)}
	bind:open={isOpen}
	onOpenChange={handleOpenChange}
	{...rest}
>
	{@render children?.()}
</Collapsible>
