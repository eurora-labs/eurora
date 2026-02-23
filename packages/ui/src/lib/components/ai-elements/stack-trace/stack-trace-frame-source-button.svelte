<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLButtonElement> {
		filePath: string;
		lineNumber?: number;
		columnNumber?: number;
		onFilePathClick?: (filePath: string, line?: number, column?: number) => void;
		children?: Snippet;
	}

	let {
		class: className,
		filePath,
		lineNumber,
		columnNumber,
		onFilePathClick,
		children,
		...rest
	}: Props = $props();

	function handleClick() {
		onFilePathClick?.(filePath, lineNumber, columnNumber);
	}
</script>

<button
	data-slot="stack-trace-frame-source-button"
	class={cn(
		'underline decoration-dotted hover:text-primary',
		onFilePathClick && 'cursor-pointer',
		className,
	)}
	disabled={!onFilePathClick}
	onclick={handleClick}
	type="button"
	{...rest}
>
	{#if children}
		{@render children()}
	{:else}
		{filePath}{#if lineNumber !== undefined}:{lineNumber}{/if}{#if columnNumber !== undefined}:{columnNumber}{/if}
	{/if}
</button>
