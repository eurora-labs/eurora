<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/button/index.js';
	import Check from '@lucide/svelte/icons/check';
	import Copy from '@lucide/svelte/icons/copy';

	let {
		class: className,
		hash,
		oncopy: onCopyCallback,
		onerror: onErrorCallback,
		timeout = 2000,
		children,
		...restProps
	}: {
		class?: string;
		hash: string;
		oncopy?: () => void;
		onerror?: (error: Error) => void;
		timeout?: number;
		children?: Snippet;
	} & Record<string, unknown> = $props();

	let isCopied = $state(false);
	let timeoutId: ReturnType<typeof setTimeout> | undefined;

	async function copyToClipboard() {
		if (typeof window === 'undefined' || !navigator?.clipboard?.writeText) {
			onErrorCallback?.(new Error('Clipboard API not available'));
			return;
		}
		try {
			if (!isCopied) {
				await navigator.clipboard.writeText(hash);
				isCopied = true;
				onCopyCallback?.();
				clearTimeout(timeoutId);
				timeoutId = setTimeout(() => (isCopied = false), timeout);
			}
		} catch (error) {
			onErrorCallback?.(error as Error);
		}
	}
</script>

<Button
	data-slot="commit-copy-button"
	class={cn('size-7 shrink-0', className)}
	onclick={copyToClipboard}
	size="icon"
	variant="ghost"
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else if isCopied}
		<Check size={14} />
	{:else}
		<Copy size={14} />
	{/if}
</Button>
