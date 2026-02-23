<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/button/index.js';
	import { getCodeBlockContext } from './code-block-context.svelte.js';
	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';

	interface Props {
		onCopy?: () => void;
		onError?: (error: Error) => void;
		timeout?: number;
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { onCopy, onError, timeout = 2000, class: className, children, ...rest }: Props = $props();

	let ctx = getCodeBlockContext();
	let isCopied = $state(false);
	let timeoutId: ReturnType<typeof setTimeout> | undefined;

	async function copyToClipboard() {
		if (typeof window === 'undefined' || !navigator?.clipboard?.writeText) {
			onError?.(new Error('Clipboard API not available'));
			return;
		}

		try {
			if (!isCopied) {
				await navigator.clipboard.writeText(ctx.code);
				isCopied = true;
				onCopy?.();
				timeoutId = setTimeout(() => {
					isCopied = false;
				}, timeout);
			}
		} catch (error) {
			onError?.(error as Error);
		}
	}

	$effect(() => {
		return () => {
			if (timeoutId) {
				clearTimeout(timeoutId);
			}
		};
	});
</script>

<Button
	data-slot="code-block-copy-button"
	class={cn('shrink-0', className)}
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
