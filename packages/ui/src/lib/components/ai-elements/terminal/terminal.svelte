<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { TerminalState, setTerminalContext } from './terminal-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		output?: string;
		isStreaming?: boolean;
		autoScroll?: boolean;
		onClear?: () => void;
		children?: Snippet;
	}

	let {
		class: className,
		output = '',
		isStreaming = false,
		autoScroll = true,
		onClear,
		children,
		...rest
	}: Props = $props();

	let ctx = new TerminalState({ output, isStreaming, autoScroll, onClear });
	setTerminalContext(ctx);

	$effect(() => {
		ctx.output = output;
	});

	$effect(() => {
		ctx.isStreaming = isStreaming;
	});

	$effect(() => {
		ctx.autoScroll = autoScroll;
	});

	$effect(() => {
		ctx.onClear = onClear;
	});
</script>

<div
	data-slot="terminal"
	class={cn(
		'flex flex-col overflow-hidden rounded-lg border bg-zinc-950 text-zinc-100',
		className,
	)}
	{...rest}
>
	{@render children?.()}
</div>
