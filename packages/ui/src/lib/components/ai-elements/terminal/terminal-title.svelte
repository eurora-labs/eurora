<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import TerminalIcon from '@lucide/svelte/icons/terminal';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<div
	data-slot="terminal-title"
	class={cn('flex items-center gap-2 text-sm text-zinc-400', className)}
	{...rest}
>
	{#if children}
		{@render children()}
	{:else}
		<TerminalIcon class="size-4" />
		Terminal
	{/if}
</div>
