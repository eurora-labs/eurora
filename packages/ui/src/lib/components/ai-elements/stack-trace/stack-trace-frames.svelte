<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getStackTraceContext } from './stack-trace-context.svelte.js';
	import type { StackFrame } from './parse-stack.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		showInternalFrames?: boolean;
		children?: Snippet;
	}

	let { class: className, showInternalFrames = true, children, ...rest }: Props = $props();

	let ctx = getStackTraceContext();

	let framesToShow = $derived(
		showInternalFrames
			? ctx.trace.frames
			: ctx.trace.frames.filter((f) => !f.isInternal),
	);

	function handleFilePathClick(frame: StackFrame) {
		if (frame.filePath && ctx.onFilePathClick) {
			ctx.onFilePathClick(
				frame.filePath,
				frame.lineNumber ?? undefined,
				frame.columnNumber ?? undefined,
			);
		}
	}
</script>

<div
	data-slot="stack-trace-frames"
	class={cn('space-y-1 p-3', className)}
	{...rest}
>
	{#if children}
		{@render children()}
	{:else if framesToShow.length === 0}
		<div class="text-muted-foreground text-xs">No stack frames</div>
	{:else}
		{#each framesToShow as frame (frame.raw)}
			<div
				class={cn(
					'text-xs',
					frame.isInternal ? 'text-muted-foreground/50' : 'text-foreground/90',
				)}
			>
				<span class="text-muted-foreground">at </span>
				{#if frame.functionName}
					<span class={frame.isInternal ? '' : 'text-foreground'}>
						{frame.functionName}{' '}
					</span>
				{/if}
				{#if frame.filePath}
					<span class="text-muted-foreground">(</span>
					<button
						class={cn(
							'underline decoration-dotted hover:text-primary',
							ctx.onFilePathClick && 'cursor-pointer',
						)}
						disabled={!ctx.onFilePathClick}
						onclick={() => handleFilePathClick(frame)}
						type="button"
					>
						{frame.filePath}{#if frame.lineNumber !== null}:{frame.lineNumber}{/if}{#if frame.columnNumber !== null}:{frame.columnNumber}{/if}
					</button>
					<span class="text-muted-foreground">)</span>
				{:else if !frame.functionName}
					<span>{frame.raw.replace(/^at\s+/, '')}</span>
				{/if}
			</div>
		{/each}
	{/if}
</div>
