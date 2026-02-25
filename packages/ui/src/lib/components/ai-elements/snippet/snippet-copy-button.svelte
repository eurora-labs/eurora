<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { InputGroupButton } from '$lib/components/input-group/index.js';
	import Check from '@lucide/svelte/icons/check';
	import Copy from '@lucide/svelte/icons/copy';
	import { useSnippet } from './snippet-context.svelte.js';

	let {
		class: className,
		oncopy: onCopyCallback,
		onerror: onErrorCallback,
		timeout = 2000,
		children,
		...restProps
	}: {
		class?: string;
		oncopy?: () => void;
		onerror?: (error: Error) => void;
		timeout?: number;
		children?: Snippet;
	} & Record<string, unknown> = $props();

	const ctx = useSnippet();

	let isCopied = $state(false);
	let timeoutId: ReturnType<typeof setTimeout> | undefined;

	async function copyToClipboard() {
		if (typeof window === 'undefined' || !navigator?.clipboard?.writeText) {
			onErrorCallback?.(new Error('Clipboard API not available'));
			return;
		}
		try {
			if (!isCopied) {
				await navigator.clipboard.writeText(ctx.code);
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

<InputGroupButton
	data-slot="snippet-copy-button"
	class={cn(className)}
	onclick={copyToClipboard}
	aria-label="Copy"
	title="Copy"
	size="icon-sm"
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else if isCopied}
		<Check class="size-3.5" />
	{:else}
		<Copy class="size-3.5" />
	{/if}
</InputGroupButton>
