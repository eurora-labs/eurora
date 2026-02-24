<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';
	import { cn } from '$lib/utils.js';
	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import { getTerminalContext } from './terminal-context.svelte.js';

	interface Props extends ButtonProps {
		onCopy?: () => void;
		onError?: (error: Error) => void;
		timeout?: number;
		children?: Snippet;
	}

	let { class: className, onCopy, onError, timeout = 2000, children, ...rest }: Props = $props();

	let ctx = getTerminalContext();
	let isCopied = $state(false);
	let timeoutId: ReturnType<typeof setTimeout> | undefined;

	async function copyToClipboard() {
		if (typeof window === 'undefined' || !navigator?.clipboard?.writeText) {
			onError?.(new Error('Clipboard API not available'));
			return;
		}

		try {
			await navigator.clipboard.writeText(ctx.output);
			isCopied = true;
			onCopy?.();
			clearTimeout(timeoutId);
			timeoutId = setTimeout(() => (isCopied = false), timeout);
		} catch (error) {
			onError?.(error as Error);
		}
	}
</script>

<Button
	data-slot="terminal-copy-button"
	class={cn('size-7 shrink-0 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100', className)}
	onclick={copyToClipboard}
	size="icon"
	variant="ghost"
	{...rest}
>
	{#if children}
		{@render children()}
	{:else if isCopied}
		<CheckIcon size={14} />
	{:else}
		<CopyIcon size={14} />
	{/if}
</Button>
