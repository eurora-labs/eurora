<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';
	import { cn } from '$lib/utils.js';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import { getTerminalContext } from './terminal-context.svelte.js';

	interface Props extends ButtonProps {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getTerminalContext();
</script>

{#if ctx.onClear}
	<Button
		data-slot="terminal-clear-button"
		class={cn(
			'size-7 shrink-0 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100',
			className,
		)}
		onclick={ctx.onClear}
		size="icon"
		variant="ghost"
		{...rest}
	>
		{#if children}
			{@render children()}
		{:else}
			<Trash2Icon size={14} />
		{/if}
	</Button>
{/if}
