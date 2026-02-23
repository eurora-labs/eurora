<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getTerminalContext } from './terminal-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		content?: Snippet;
		children?: Snippet;
	}

	let { class: className, content, children, ...rest }: Props = $props();

	let ctx = getTerminalContext();
	let containerEl: HTMLDivElement | undefined = $state();

	$effect(() => {
		ctx.output;
		if (ctx.autoScroll && containerEl) {
			containerEl.scrollTop = containerEl.scrollHeight;
		}
	});
</script>

<div
	data-slot="terminal-content"
	class={cn('max-h-96 overflow-auto p-4 font-mono text-sm leading-relaxed', className)}
	bind:this={containerEl}
	{...rest}
>
	{#if children}
		{@render children()}
	{:else if content}
		<pre class="whitespace-pre-wrap break-words">{@render content()}{#if ctx.isStreaming}<span
					class="ml-0.5 inline-block h-4 w-2 animate-pulse bg-zinc-100"
				></span>{/if}</pre>
	{:else}
		<pre class="whitespace-pre-wrap break-words">{ctx.output}{#if ctx.isStreaming}<span
					class="ml-0.5 inline-block h-4 w-2 animate-pulse bg-zinc-100"
				></span>{/if}</pre>
	{/if}
</div>
